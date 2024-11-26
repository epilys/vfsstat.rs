// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::{boxed::Box, format, string::String};
use core::{convert::TryInto, pin::Pin};

use log::debug;

use crate::{
    sqlite3ext::{
        sqlite3_file, sqlite3_int64, sqlite3_io_methods, sqlite3_vfs, SQLITE_FCNTL_VFSNAME,
        SQLITE_OK, SQLITE_OPEN_MAIN_DB, SQLITE_OPEN_MAIN_JOURNAL, SQLITE_OPEN_MASTER_JOURNAL,
        SQLITE_OPEN_SUBJOURNAL, SQLITE_OPEN_TEMP_DB, SQLITE_OPEN_TEMP_JOURNAL, SQLITE_OPEN_WAL,
    },
    statcnt, FileStats, FileType,
};

#[repr(C)]
pub struct Vfs {
    inner: sqlite3_vfs,
    parent: core::ptr::NonNull<sqlite3_vfs>,
    pub file_stats: FileStats,
}

impl Drop for Vfs {
    fn drop(&mut self) {}
}

#[repr(C)]
pub struct StatConn {
    base: sqlite3_file,
    filetype: FileType,
    vfs: core::ptr::NonNull<Vfs>,
    real: sqlite3_file,
}

#[no_mangle]
pub static STAT_IO_METHODS: sqlite3_io_methods = sqlite3_io_methods {
    iVersion: 3,
    xClose: Some(stat_close),
    xRead: Some(stat_read),
    xWrite: Some(stat_write),
    xTruncate: Some(stat_truncate),
    xSync: Some(stat_sync),
    xFileSize: Some(stat_file_size),
    xLock: Some(stat_lock),
    xUnlock: Some(stat_unlock),
    xCheckReservedLock: Some(stat_check_reserved_lock),
    xFileControl: Some(stat_file_control),
    xSectorSize: Some(stat_sector_size),
    xDeviceCharacteristics: Some(stat_device_characteristics),
    xShmMap: Some(stat_shm_map),
    xShmLock: Some(stat_shm_lock),
    xShmBarrier: Some(stat_shm_barrier),
    xShmUnmap: Some(stat_shm_unmap),
    xFetch: Some(stat_fetch),
    xUnfetch: Some(stat_unfetch),
};

#[no_mangle]
pub unsafe extern "C" fn stat_close(arg1: *mut sqlite3_file) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    if !stat_conn_ref.real.pMethods.is_null() {
        return ((*stat_conn_ref.real.pMethods).xClose.unwrap())(&mut stat_conn_ref.real as *mut _);
    }

    SQLITE_OK as i32
}

#[no_mangle]
pub unsafe extern "C" fn stat_read(
    arg1: *mut sqlite3_file,
    arg2: *mut ::core::ffi::c_void,
    iAmt: ::core::ffi::c_int,
    iOfst: sqlite3_int64,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Read
    ) += 1;
    let ret = ((*stat_conn_ref.real.pMethods).xRead.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        arg2,
        iAmt,
        iOfst,
    );
    if ret == SQLITE_OK as i32 {
        *statcnt!(
            mut stat_conn_ref.vfs.as_mut().file_stats,
            stat_conn_ref.filetype,
            BytesIn
        ) += iAmt as u64;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn stat_write(
    arg1: *mut sqlite3_file,
    arg2: *const ::core::ffi::c_void,
    iAmt: ::core::ffi::c_int,
    iOfst: sqlite3_int64,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Write
    ) += 1;
    let ret = ((*stat_conn_ref.real.pMethods).xWrite.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        arg2,
        iAmt,
        iOfst,
    );
    if ret == SQLITE_OK as i32 {
        *statcnt!(
            mut stat_conn_ref.vfs.as_mut().file_stats,
            stat_conn_ref.filetype,
            BytesOut
        ) += iAmt as u64;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn stat_truncate(
    arg1: *mut sqlite3_file,
    size: sqlite3_int64,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xTruncate.unwrap())(&mut stat_conn_ref.real as *mut _, size)
}

#[no_mangle]
pub unsafe extern "C" fn stat_sync(
    arg1: *mut sqlite3_file,
    flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Sync
    ) += 1;
    ((*stat_conn_ref.real.pMethods).xSync.unwrap())(&mut stat_conn_ref.real as *mut _, flags)
}

#[no_mangle]
pub unsafe extern "C" fn stat_file_size(
    arg1: *mut sqlite3_file,
    pSize: *mut sqlite3_int64,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xFileSize.unwrap())(&mut stat_conn_ref.real as *mut _, pSize)
}

#[no_mangle]
pub unsafe extern "C" fn stat_lock(
    arg1: *mut sqlite3_file,
    arg2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Lock
    ) += 1;
    ((*stat_conn_ref.real.pMethods).xLock.unwrap())(&mut stat_conn_ref.real as *mut _, arg2)
}

#[no_mangle]
pub unsafe extern "C" fn stat_unlock(
    arg1: *mut sqlite3_file,
    arg2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Lock
    ) += 1;
    ((*stat_conn_ref.real.pMethods).xUnlock.unwrap())(&mut stat_conn_ref.real as *mut _, arg2)
}

#[no_mangle]
pub unsafe extern "C" fn stat_check_reserved_lock(
    arg1: *mut sqlite3_file,
    pResOut: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    *statcnt!(
        mut stat_conn_ref.vfs.as_mut().file_stats,
        stat_conn_ref.filetype,
        Lock
    ) += 1;
    ((*stat_conn_ref.real.pMethods).xCheckReservedLock.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        pResOut,
    )
}

#[no_mangle]
pub unsafe extern "C" fn stat_file_control(
    arg1: *mut sqlite3_file,
    op: ::core::ffi::c_int,
    pArg: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    let rc = ((*stat_conn_ref.real.pMethods).xFileControl.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        op,
        pArg,
    );
    if rc == SQLITE_OK as i32 && op == SQLITE_FCNTL_VFSNAME as i32 {
        // TODO:
        //  *(char**)pArg = sqlite3_mprintf("vstat/%z", *(char**)pArg);
        debug!("rc == SQLITE_OK as i32 && op == SQLITE_FCNTL_VFSNAME as i32");
    }
    rc
}

#[no_mangle]
pub unsafe extern "C" fn stat_sector_size(arg1: *mut sqlite3_file) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xSectorSize.unwrap())(&mut stat_conn_ref.real as *mut _)
}

unsafe extern "C" fn stat_device_characteristics(arg1: *mut sqlite3_file) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods)
        .xDeviceCharacteristics
        .unwrap())(&mut stat_conn_ref.real as *mut _)
}

unsafe extern "C" fn stat_shm_map(
    arg1: *mut sqlite3_file,
    iPg: ::core::ffi::c_int,
    pgsz: ::core::ffi::c_int,
    arg2: ::core::ffi::c_int,
    arg3: *mut *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xShmMap.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        iPg,
        pgsz,
        arg2,
        arg3,
    )
}

unsafe extern "C" fn stat_shm_lock(
    arg1: *mut sqlite3_file,
    offset: ::core::ffi::c_int,
    n: ::core::ffi::c_int,
    flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xShmLock.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        offset,
        n,
        flags,
    )
}

unsafe extern "C" fn stat_shm_barrier(arg1: *mut sqlite3_file) {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xShmBarrier.unwrap())(&mut stat_conn_ref.real as *mut _)
}
unsafe extern "C" fn stat_shm_unmap(
    arg1: *mut sqlite3_file,
    deleteFlag: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xShmUnmap.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        deleteFlag,
    )
}

unsafe extern "C" fn stat_fetch(
    arg1: *mut sqlite3_file,
    iOfst: sqlite3_int64,
    iAmt: ::core::ffi::c_int,
    pp: *mut *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xFetch.unwrap())(
        &mut stat_conn_ref.real as *mut _,
        iOfst,
        iAmt,
        pp,
    )
}

unsafe extern "C" fn stat_unfetch(
    arg1: *mut sqlite3_file,
    iOfst: sqlite3_int64,
    p: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(arg1 as *mut StatConn).expect("null file_ptr in stat_unfetch");
    let stat_conn_ref = stat_conn.as_mut();
    ((*stat_conn_ref.real.pMethods).xUnfetch.unwrap())(&mut stat_conn_ref.real as *mut _, iOfst, p)
}

#[no_mangle]
pub unsafe extern "C" fn stat_open(
    vfs: *mut sqlite3_vfs,
    zPath: *const ::core::ffi::c_char,
    file_ptr: *mut sqlite3_file,
    flags: ::core::ffi::c_int,
    pOutFlags: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(vfs) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();

    let mut stat_conn: core::ptr::NonNull<StatConn> =
        core::ptr::NonNull::new(file_ptr as *mut StatConn).expect("null file_ptr in stat_open");
    let stat_conn_ref = stat_conn.as_mut();
    let parent_open = (vfs_.parent.as_ref().xOpen.unwrap())(
        vfs_.parent.as_ptr() as _,
        zPath,
        &mut stat_conn_ref.real as *mut _,
        flags,
        pOutFlags,
    );
    if (flags & SQLITE_OPEN_MAIN_DB as i32) > 0 {
        stat_conn_ref.filetype = FileType::Main;
    } else if (flags & SQLITE_OPEN_MAIN_JOURNAL as i32) > 0 {
        stat_conn_ref.filetype = FileType::Journal;
    } else if (flags & SQLITE_OPEN_WAL as i32) > 0 {
        stat_conn_ref.filetype = FileType::Wal;
    } else if (flags & SQLITE_OPEN_MASTER_JOURNAL as i32) > 0 {
        stat_conn_ref.filetype = FileType::MasterJournal;
    } else if (flags & SQLITE_OPEN_SUBJOURNAL as i32) > 0 {
        stat_conn_ref.filetype = FileType::SubJournal;
    } else if (flags & SQLITE_OPEN_TEMP_DB as i32) > 0 {
        stat_conn_ref.filetype = FileType::TempDb;
    } else if (flags & SQLITE_OPEN_TEMP_JOURNAL as i32) > 0 {
        stat_conn_ref.filetype = FileType::TempJournal;
    } else {
        stat_conn_ref.filetype = FileType::Transient;
    }
    *statcnt!(mut vfs_.file_stats, stat_conn_ref.filetype, Open) += 1;

    if parent_open == SQLITE_OK as _ {
        stat_conn_ref.base.pMethods = &STAT_IO_METHODS;
    } else {
        stat_conn_ref.base.pMethods = core::ptr::null_mut();
    }
    stat_conn_ref.vfs = vfs_ptr;
    parent_open
}

#[no_mangle]
pub unsafe extern "C" fn stat_delete(
    vfs: *mut sqlite3_vfs,
    zName: *const ::core::ffi::c_char,
    syncDir: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(vfs) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();

    *statcnt!(mut vfs_.file_stats, FileType::Any, Delete) += 1;
    (vfs_.parent.as_ref().xDelete.unwrap())(vfs_.parent.as_ptr() as _, zName, syncDir)
}

#[no_mangle]
pub unsafe extern "C" fn stat_access(
    vfs: *mut sqlite3_vfs,
    zName: *const ::core::ffi::c_char,
    flags: ::core::ffi::c_int,
    pResOut: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(vfs) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, Access) += 1;
    (vfs_.parent.as_ref().xAccess.unwrap())(vfs_.parent.as_ptr() as _, zName, flags, pResOut)
}

unsafe extern "C" fn stat_full_pathname(
    arg1: *mut sqlite3_vfs,
    zName: *const ::core::ffi::c_char,
    nOut: ::core::ffi::c_int,
    zOut: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, FullPath) += 1;
    (vfs_.parent.as_ref().xFullPathname.unwrap())(vfs_.parent.as_ptr() as _, zName, nOut, zOut)
}

unsafe extern "C" fn stat_dlopen(
    arg1: *mut sqlite3_vfs,
    zFilename: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_void {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    (vfs_.parent.as_ref().xDlOpen.unwrap())(vfs_.parent.as_ptr() as _, zFilename)
}

unsafe extern "C" fn stat_dlerror(
    arg1: *mut sqlite3_vfs,
    nByte: ::core::ffi::c_int,
    zErrMsg: *mut ::core::ffi::c_char,
) {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    (vfs_.parent.as_ref().xDlError.unwrap())(vfs_.parent.as_ptr() as _, nByte, zErrMsg)
}

unsafe extern "C" fn stat_dlsym(
    arg1: *mut sqlite3_vfs,
    arg2: *mut ::core::ffi::c_void,
    zSymbol: *const ::core::ffi::c_char,
) -> ::core::option::Option<
    unsafe extern "C" fn(
        arg1: *mut sqlite3_vfs,
        arg2: *mut ::core::ffi::c_void,
        zSymbol: *const ::core::ffi::c_char,
    ),
> {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    (vfs_.parent.as_ref().xDlSym.unwrap())(vfs_.parent.as_ptr() as _, arg2, zSymbol)
}

unsafe extern "C" fn stat_dlclose(arg1: *mut sqlite3_vfs, arg2: *mut ::core::ffi::c_void) {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    (vfs_.parent.as_ref().xDlClose.unwrap())(vfs_.parent.as_ptr() as _, arg2)
}

unsafe extern "C" fn stat_randomness(
    arg1: *mut sqlite3_vfs,
    nByte: ::core::ffi::c_int,
    zOut: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, Random) += 1;
    (vfs_.parent.as_ref().xRandomness.unwrap())(vfs_.parent.as_ptr() as _, nByte, zOut)
}

unsafe extern "C" fn stat_sleep(
    arg1: *mut sqlite3_vfs,
    microseconds: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, Sleep) += 1;
    (vfs_.parent.as_ref().xSleep.unwrap())(vfs_.parent.as_ptr() as _, microseconds)
}

unsafe extern "C" fn stat_current_time(
    arg1: *mut sqlite3_vfs,
    arg2: *mut f64,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, CurrentTime) += 1;
    (vfs_.parent.as_ref().xCurrentTime.unwrap())(vfs_.parent.as_ptr() as _, arg2)
}

unsafe extern "C" fn stat_get_last_error(
    arg1: *mut sqlite3_vfs,
    arg2: ::core::ffi::c_int,
    arg3: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    (vfs_.parent.as_ref().xGetLastError.unwrap())(vfs_.parent.as_ptr() as _, arg2, arg3)
}

unsafe extern "C" fn stat_current_time_int64(
    arg1: *mut sqlite3_vfs,
    arg2: *mut sqlite3_int64,
) -> ::core::ffi::c_int {
    let mut vfs_ptr = if let Some(ptr) = core::ptr::NonNull::new(arg1) {
        ptr
    } else {
        panic!("Could not find default sqlite3 vfs");
    };
    let sqlite_vfs: &mut sqlite3_vfs = vfs_ptr.as_mut();
    let mut vfs_ptr: core::ptr::NonNull<Vfs> =
        core::ptr::NonNull::new(sqlite_vfs.pAppData as *mut Vfs)
            .expect("pAppData of stat vfs is null");
    let vfs_ = vfs_ptr.as_mut();
    *statcnt!(mut vfs_.file_stats, FileType::Any, CurrentTime) += 1;
    (vfs_.parent.as_ref().xCurrentTimeInt64.unwrap())(vfs_.parent.as_ptr() as _, arg2)
}

pub const VFS_NAME: &[u8] = b"vfsstat_rs\0";

impl Vfs {
    pub fn new() -> Result<Pin<Box<Self>>, String> {
        let default_ptr = unsafe { ((*crate::API).vfs_find.unwrap())(core::ptr::null()) };
        let default = if let Some(default) = core::ptr::NonNull::new(default_ptr) {
            default
        } else {
            return Err("Could not find default sqlite3 vfs".into());
        };

        let default_ref = unsafe { default.as_ref() };
        let mut inner = *default_ref;
        debug!(
            "default vfs name: {:?} ",
            unsafe { core::ffi::CStr::from_ptr(inner.zName) }.to_str()
        );
        inner.iVersion = 2;
        inner.zName = VFS_NAME.as_ptr() as _;
        inner.pNext = core::ptr::null_mut();
        inner.pAppData = core::ptr::null_mut();
        inner.xOpen = Some(stat_open);
        inner.xDelete = Some(stat_delete);
        inner.xAccess = Some(stat_access);
        inner.xFullPathname = Some(stat_full_pathname);
        inner.xDlOpen = Some(stat_dlopen);
        inner.xDlError = Some(stat_dlerror);
        inner.xDlSym = Some(stat_dlsym);
        inner.xDlClose = Some(stat_dlclose);
        inner.xRandomness = Some(stat_randomness);
        inner.xSleep = Some(stat_sleep);
        inner.xCurrentTime = Some(stat_current_time);
        inner.xCurrentTimeInt64 = Some(stat_current_time_int64);
        inner.xGetLastError = Some(stat_get_last_error);
        let fsize: i32 = core::mem::size_of::<StatConn>()
            .try_into()
            .expect("Could not convert VFS file size from usize to i32");
        inner.szOsFile += fsize;

        let mut self_ = Box::pin(Vfs {
            parent: default,
            inner,
            file_stats: FileStats::default(),
        });
        self_.inner.pAppData =
            unsafe { core::mem::transmute((self_.as_ref().get_ref()) as *const _) };
        let ret = unsafe { ((*crate::API).vfs_register.unwrap())(&mut self_.inner, 1) };
        if ret != SQLITE_OK as _ {
            return Err(format!("Vfs::new() sqlite3_vfs_register returned {}", ret,));
        }
        Ok(self_)
    }
}
