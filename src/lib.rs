// SPDX-License-Identifier: GPL-3.0-or-later

// Uncomment to compile without `std`, but be aware this has consequences for
// panicking (see panic handler later in this file)
//
// #![no_std]
#![allow(
    non_snake_case,
    dead_code,
    non_camel_case_types,
    non_upper_case_globals
)]

extern crate alloc;

#[allow(clippy::type_complexity)]
pub(crate) mod sqlite3ext;

use alloc::{ffi::CString, string::String};
use core::convert::TryInto;

use log::{debug, trace};
use sqlite3ext::{
    sqlite3, sqlite3_api_routines, SQLITE_ERROR, SQLITE_OK, SQLITE_OK_LOAD_PERMANENTLY,
};

pub mod vfs;
pub mod vtab;

static mut API: *mut sqlite3_api_routines = core::ptr::null_mut();

// If you build with no_std, you can use the following panic handler and global
// allocator definitions.
//
// Be aware that core is built with unwinding panicking, so you have to compile
// core yourself too if you don't include std in your cdylib. Otherwise you will
// get many "symbols missing" errors. Such is life.
//
// extern "C" {
//     fn printf(fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
// }
//
// #[panic_handler]
// fn panic(info: &core::panic::PanicInfo) -> ! {
//     let error = alloc::format!("{}", info);
//     if let Ok(err_c_s) = CString::new(error) {
//         const FMT_STRING: &[core::ffi::c_char] = &[
//             '%' as core::ffi::c_char,
//             's' as core::ffi::c_char,
//             '\n' as core::ffi::c_char,
//         ];
//         unsafe { printf(FMT_STRING.as_ptr(), err_c_s.as_ptr()) };
//     }

//     loop {}
// }

// #[repr(C)]
// struct Sqlite3Allocator {
//     _unused: [u8; 0],
// }

// #[global_allocator]
// static ALLOCATOR: Sqlite3Allocator = Sqlite3Allocator { _unused: [0; 0] };

// unsafe impl Sync for Sqlite3Allocator {}

// unsafe impl core::alloc::GlobalAlloc for Sqlite3Allocator {
//     unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
//         let size = layout.size();
//         #[cfg(target_pointer_width = "32")]
//         {
//             crate::sqlite3ext::sqlite3_malloc(
//                 size.try_into()
//                     .expect("Cannot fit allocation size into i32"),
//             )
//             .cast()
//         }
//         #[cfg(not(target_pointer_width = "32"))]
//         {
//             crate::sqlite3ext::sqlite3_malloc64(
//                 size.try_into()
//                     .expect("Cannot fit allocation size into u64"),
//             )
//             .cast()
//         }
//     }
//     unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
//         crate::sqlite3ext::sqlite3_free(ptr.cast());
//     }
// }

/// File types
#[repr(C)]
#[derive(Copy, Clone)]
pub enum FileType {
    /// Main database file
    Main = 0,
    /// Rollback journal
    Journal = 1,
    /// Write-ahead log file
    Wal = 2,
    /// Master journal
    MasterJournal = 3,
    /// Subjournal
    SubJournal = 4,
    /// TEMP database
    TempDb = 5,
    /// Journal for TEMP database
    TempJournal = 6,
    /// Transient database
    Transient = 7,
    /// Unspecified file type
    Any = 8,
}

/// Stat types
#[repr(C)]
#[derive(Debug, Default)]
pub struct Stats {
    /// 0,   Bytes read in
    BytesIn: u64,
    /// 1,   Bytes written out
    BytesOut: u64,
    /// 2,   Read requests
    Read: u64,
    /// 3,   Write requests
    Write: u64,
    /// 4,   Syncs
    Sync: u64,
    /// 5,   File opens
    Open: u64,
    /// 6,   Lock requests
    Lock: u64,
    /// 7,   xAccess calls.  filetype==ANY only
    Access: u64,
    /// 8,   xDelete calls.  filetype==ANY only
    Delete: u64,
    /// 9,   xFullPathname calls.  ANY only
    FullPath: u64,
    /// 10,   xRandomness calls.    ANY only
    Random: u64,
    /// 11,   xSleep calls.         ANY only
    Sleep: u64,
    /// 12,   xCurrentTime calls.   ANY only
    CurrentTime: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum StatField {
    /// Bytes read in
    BytesIn = 0,
    /// Bytes written out
    BytesOut = 1,
    /// Read requests
    Read = 2,
    /// Write requests
    Write = 3,
    /// Syncs
    Sync = 4,
    /// File opens
    Open = 5,
    /// Lock requests
    Lock = 6,
    /// xAccess calls.  filetype==ANY only
    Access = 7,
    /// xDelete calls.  filetype==ANY only
    Delete = 8,
    /// xFullPathname calls.  ANY only
    FullPath = 9,
    /// xRandomness calls.    ANY only
    Random = 10,
    /// xSleep calls.         ANY only
    Sleep = 11,
    /// xCurrentTime calls.   ANY only
    CurrentTime = 12,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FileStats {
    /// Main database file
    main: Stats,
    /// Rollback journal
    journal: Stats,
    /// Write-ahead log file
    wal: Stats,
    /// Master journal
    master_journal: Stats,
    /// Subjournal
    sub_journal: Stats,
    /// TEMP database
    temp_db: Stats,
    /// Journal for TEMP database
    temp_journal: Stats,
    /// Transient database
    transient: Stats,
    /// Unspecified file type
    any: Stats,
}

#[macro_export]
macro_rules! statcnt {
    (mut $filestats:expr, $filetype:expr, $field:ident) => {{
        match $filetype {
            FileType::Main => &mut $filestats.main.$field,
            FileType::Journal => &mut $filestats.journal.$field,
            FileType::Wal => &mut $filestats.wal.$field,
            FileType::MasterJournal => &mut $filestats.master_journal.$field,
            FileType::SubJournal => &mut $filestats.sub_journal.$field,
            FileType::TempDb => &mut $filestats.temp_db.$field,
            FileType::TempJournal => &mut $filestats.temp_journal.$field,
            FileType::Transient => &mut $filestats.transient.$field,
            FileType::Any => &mut $filestats.any.$field,
        }
    }};
    ($filestats:expr, $filetype:expr, $field:ident) => {{
        match $filetype {
            FileType::Main => &$filestats.main.$field,
            FileType::Journal => &$filestats.journal.$field,
            FileType::Wal => &$filestats.wal.$field,
            FileType::MasterJournal => &$filestats.master_journal.$field,
            FileType::SubJournal => &$filestats.sub_journal.$field,
            FileType::TempDb => &$filestats.temp_db.$field,
            FileType::TempJournal => &$filestats.temp_journal.$field,
            FileType::Transient => &$filestats.transient.$field,
            FileType::Any => &$filestats.any.$field,
        }
    }};
}

fn err_to_sqlite3_str(err: String) -> Option<*mut ::core::ffi::c_char> {
    let err_s = CString::new(err).ok()?;
    let len = err_s.as_bytes_with_nul().len();
    let ptr: *mut ::core::ffi::c_char =
        unsafe { ((*API).malloc.unwrap())(len.try_into().ok()?) } as _;
    if !ptr.is_null() {
        unsafe { core::ptr::copy_nonoverlapping(err_s.as_ptr(), ptr, len) };
        Some(ptr)
    } else {
        debug!("err_to_sqlite3_str(): sqlite3_malloc returned null");
        None
    }
}

#[no_mangle]
pub unsafe extern "C" fn vtab_register(
    db: *mut sqlite3,
    pzErrMsg: *mut *mut ::core::ffi::c_char,
    _pApi: *mut sqlite3_api_routines,
) -> ::core::ffi::c_int {
    if let Err(err) = vtab::VTab::create(db) {
        debug!("vtab::new() returned: {}", &err);
        if let Some(ptr) = err_to_sqlite3_str(err) {
            *pzErrMsg = ptr;
        }
        return SQLITE_ERROR as _;
    }
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn sqlite3_vfsstatrs_init(
    db: *mut sqlite3,
    pzErrMsg: *mut *mut ::core::ffi::c_char,
    pApi: *mut sqlite3_api_routines,
) -> ::core::ffi::c_int {
    trace!("sqlite3_vfsstat_rs_init");
    API = pApi;

    let vfs = match vfs::Vfs::new() {
        Err(err) => {
            debug!("vfs::new() returned: {}", &err);
            if let Some(ptr) = err_to_sqlite3_str(err) {
                *pzErrMsg = ptr;
            }
            return SQLITE_ERROR as _;
        }
        Ok(v) => v,
    };
    core::mem::forget(vfs);
    let ret = vtab_register(db, pzErrMsg, pApi);
    if ret != SQLITE_OK as _ {
        return ret;
    } else {
        let ret = ((*pApi).auto_extension.unwrap())(Some(core::mem::transmute::<
            *const (),
            unsafe extern "C" fn(),
        >(vtab_register as *const ())));
        if ret != SQLITE_OK as _ {
            return ret;
        }
    }

    SQLITE_OK_LOAD_PERMANENTLY as _
}
