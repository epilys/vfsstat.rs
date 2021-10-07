#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
pub(crate) mod sqlite3ext;
use log::{debug, trace};
use std::convert::TryInto;
use std::ffi::CString;

use sqlite3ext::{
    sqlite3, sqlite3_api_routines, SQLITE_ERROR, SQLITE_OK, SQLITE_OK_LOAD_PERMANENTLY,
};

/*
** File types
*/
#[repr(C)]
#[derive(Copy, Clone)]
pub enum FileType {
    Main = 0,          /* Main database file */
    Journal = 1,       /* Rollback journal */
    Wal = 2,           /* Write-ahead log file */
    MasterJournal = 3, /* Master journal */
    SubJournal = 4,    /* Subjournal */
    TempDb = 5,        /* TEMP database */
    TempJournal = 6,   /* Journal for TEMP database */
    Transient = 7,     /* Transient database */
    Any = 8,           /* Unspecified file type */
}

/*
** Stat types
*/
#[repr(C)]
#[derive(Debug, Default)]
pub struct Stats {
    BytesIn: u64,     //     0,   /* Bytes read in */
    BytesOut: u64,    //    1,   /* Bytes written out */
    Read: u64,        //        2,   /* Read requests */
    Write: u64,       //       3,   /* Write requests */
    Sync: u64,        //        4,   /* Syncs */
    Open: u64,        //        5,   /* File opens */
    Lock: u64,        //        6,   /* Lock requests */
    Access: u64,      //      7,   /* xAccess calls.  filetype==ANY only */
    Delete: u64,      //      8,   /* xDelete calls.  filetype==ANY only */
    FullPath: u64,    //    9,   /* xFullPathname calls.  ANY only */
    Random: u64,      //      10,   /* xRandomness calls.    ANY only */
    Sleep: u64,       //       11,   /* xSleep calls.         ANY only */
    CurrentTime: u64, //     12,   /* xCurrentTime calls.   ANY only */
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum StatField {
    BytesIn = 0,      /* Bytes read in */
    BytesOut = 1,     /* Bytes written out */
    Read = 2,         /* Read requests */
    Write = 3,        /* Write requests */
    Sync = 4,         /* Syncs */
    Open = 5,         /* File opens */
    Lock = 6,         /* Lock requests */
    Access = 7,       /* xAccess calls.  filetype==ANY only */
    Delete = 8,       /* xDelete calls.  filetype==ANY only */
    FullPath = 9,     /* xFullPathname calls.  ANY only */
    Random = 10,      /* xRandomness calls.    ANY only */
    Sleep = 11,       /* xSleep calls.         ANY only */
    CurrentTime = 12, /* xCurrentTime calls.   ANY only */
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FileStats {
    main: Stats,           /* Main database file */
    journal: Stats,        /* Rollback journal */
    wal: Stats,            /* Write-ahead log file */
    master_journal: Stats, /* Master journal */
    sub_journal: Stats,    /* Subjournal */
    temp_db: Stats,        /* TEMP database */
    temp_journal: Stats,   /* Journal for TEMP database */
    transient: Stats,      /* Transient database */
    any: Stats,            /* Unspecified file type */
}

macro_rules! statcnt {
    (mut $filestats:expr, $filetype:expr, $field:ident) => {{
        //std::dbg!(&$filestats);
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
        //std::dbg!(&$filestats);
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

pub mod vfs;
pub mod vtab;

fn err_to_sqlite3_str(err: String) -> Option<*mut ::std::os::raw::c_char> {
    let err_s = CString::new(err).ok()?;
    let len = err_s.as_bytes_with_nul().len();
    let ptr: *mut ::std::os::raw::c_char =
        unsafe { ((*API).malloc.unwrap())(len.try_into().ok()?) } as _;
    if !ptr.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(err_s.as_ptr(), ptr, len) };
        Some(ptr)
    } else {
        debug!("err_to_sqlite3_str(): sqlite3_malloc returned null");
        None
    }
}

static mut API: *mut sqlite3_api_routines = std::ptr::null_mut();

#[no_mangle]
pub unsafe extern "C" fn vtab_register(
    db: *mut sqlite3,
    pzErrMsg: *mut *mut ::std::os::raw::c_char,
    _pApi: *mut sqlite3_api_routines,
) -> ::std::os::raw::c_int {
    if let Err(err) = vtab::VTab::new(db) {
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
    pzErrMsg: *mut *mut ::std::os::raw::c_char,
    pApi: *mut sqlite3_api_routines,
) -> ::std::os::raw::c_int {
    use log::LevelFilter;
    #[cfg(debug_assertions)]
    let log_level = LevelFilter::Trace;
    #[cfg(not(debug_assertions))]
    let log_level = LevelFilter::Error;
    //LevelFilter::Error;
    //LevelFilter::Warn;
    //LevelFilter::Info;
    //LevelFilter::Debug;
    let _ = env_logger::builder()
        .format_timestamp_nanos()
        .filter_level(log_level)
        .try_init();

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
    std::mem::forget(vfs);
    let ret = vtab_register(db, pzErrMsg, pApi);
    if ret != SQLITE_OK as _ {
        return ret;
    } else {
        let ret = ((*pApi).auto_extension.unwrap())(Some(std::mem::transmute(
            vtab_register as *const (),
        )));
        if ret != SQLITE_OK as _ {
            return ret;
        }
    }

    SQLITE_OK_LOAD_PERMANENTLY as _
}
