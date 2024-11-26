// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::{boxed::Box, format, string::String};

use crate::{
    sqlite3ext::{
        sqlite3, sqlite3_context, sqlite3_index_info, sqlite3_int64, sqlite3_module, sqlite3_value,
        sqlite3_vfs, sqlite3_vtab, sqlite3_vtab_cursor, SQLITE_ERROR, SQLITE_OK,
    },
    statcnt, FileType, StatField,
};

#[repr(C)]
struct VfsStatCursor {
    /// Base class.  Must be first
    base: sqlite3_vtab_cursor,
    filetype: FileType,
    field: StatField,
}

#[repr(C)]
pub struct VTab {
    _unused: [u8; 0],
}

#[no_mangle]
pub unsafe extern "C" fn VtabConnect(
    db: *mut sqlite3,
    _pAux: *mut ::core::ffi::c_void,
    _argc: ::core::ffi::c_int,
    _argv: *const *const ::core::ffi::c_char,
    ppVTab: *mut *mut sqlite3_vtab,
    _pzErr: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let rc = unsafe {
        ((*crate::API).declare_vtab.unwrap())(
            db,
            b"CREATE TABLE x(file,stat,count)\0".as_ptr() as _,
        )
    };
    if rc == SQLITE_OK as _ {
        let pNew: Box<sqlite3_vtab> = Box::new(sqlite3_vtab {
            pModule: core::ptr::null_mut(),
            nRef: 0,
            zErrMsg: core::ptr::null_mut(),
        });
        unsafe { *ppVTab = Box::into_raw(pNew) };
    }
    rc
}

unsafe extern "C" fn VtabBestIndex(
    _pVTab: *mut sqlite3_vtab,
    _arg1: *mut sqlite3_index_info,
) -> ::core::ffi::c_int {
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn VtabDisconnect(pVTab: *mut sqlite3_vtab) -> ::core::ffi::c_int {
    debug_assert!(!pVTab.is_null());
    let _pNew: Box<sqlite3_vtab> = unsafe { Box::from_raw(pVTab) };
    SQLITE_OK as _
}

extern "C" fn VtabOpen(
    pVTab: *mut sqlite3_vtab,
    ppCursor: *mut *mut sqlite3_vtab_cursor,
) -> ::core::ffi::c_int {
    let cursor: Box<VfsStatCursor> = Box::new(VfsStatCursor {
        base: sqlite3_vtab_cursor { pVtab: pVTab },
        filetype: FileType::Main,
        field: StatField::BytesIn,
    });
    unsafe { *ppCursor = Box::into_raw(cursor) as _ };
    SQLITE_OK as _
}

extern "C" fn VtabClose(arg1: *mut sqlite3_vtab_cursor) -> ::core::ffi::c_int {
    debug_assert!(!arg1.is_null());
    let _cur: Box<VfsStatCursor> = unsafe { Box::from_raw(arg1 as *mut VfsStatCursor) };
    SQLITE_OK as _
}

/// Only a full table scan is supported.  So xFilter simply rewinds to the
/// beginning.
#[no_mangle]
pub unsafe extern "C" fn VtabFilter(
    arg1: *mut sqlite3_vtab_cursor,
    _idxNum: ::core::ffi::c_int,
    _idxStr: *const ::core::ffi::c_char,
    _argc: ::core::ffi::c_int,
    _argv: *mut *mut sqlite3_value,
) -> ::core::ffi::c_int {
    let mut ptr = core::ptr::NonNull::new(arg1 as *mut VfsStatCursor).unwrap();
    let cur: &mut VfsStatCursor = unsafe { ptr.as_mut() };
    cur.filetype = FileType::Main;
    cur.field = StatField::BytesIn;
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn VtabNext(arg1: *mut sqlite3_vtab_cursor) -> ::core::ffi::c_int {
    let mut ptr = core::ptr::NonNull::new(arg1 as *mut VfsStatCursor).unwrap();
    let cur: &mut VfsStatCursor = unsafe { ptr.as_mut() };
    match cur.field {
        StatField::BytesIn => {
            cur.field = StatField::BytesOut;
        }
        StatField::BytesOut => {
            cur.field = StatField::Read;
        }
        StatField::Read => {
            cur.field = StatField::Write;
        }
        StatField::Write => {
            cur.field = StatField::Sync;
        }
        StatField::Sync => {
            cur.field = StatField::Open;
        }
        StatField::Open => {
            cur.field = StatField::Lock;
        }
        StatField::Lock => {
            cur.field = StatField::Access;
        }
        StatField::Access => {
            cur.field = StatField::Delete;
        }
        StatField::Delete => {
            cur.field = StatField::FullPath;
        }
        StatField::FullPath => {
            cur.field = StatField::Random;
        }
        StatField::Random => {
            cur.field = StatField::Sleep;
        }
        StatField::Sleep => {
            cur.field = StatField::CurrentTime;
        }
        StatField::CurrentTime => {
            cur.field = StatField::BytesIn;
            match cur.filetype {
                FileType::Main => {
                    cur.filetype = FileType::Journal;
                }
                FileType::Journal => {
                    cur.filetype = FileType::Wal;
                }
                FileType::Wal => {
                    cur.filetype = FileType::MasterJournal;
                }
                FileType::MasterJournal => {
                    cur.filetype = FileType::SubJournal;
                }
                FileType::SubJournal => {
                    cur.filetype = FileType::TempDb;
                }
                FileType::TempDb => {
                    cur.filetype = FileType::TempJournal;
                }
                FileType::TempJournal => {
                    cur.filetype = FileType::Transient;
                }
                FileType::Transient => {
                    cur.filetype = FileType::Any;
                }
                FileType::Any => {
                    cur.filetype = FileType::Main;
                }
            }
        }
    }
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn VtabEof(arg1: *mut sqlite3_vtab_cursor) -> ::core::ffi::c_int {
    let mut ptr = core::ptr::NonNull::new(arg1 as *mut VfsStatCursor).unwrap();
    let cur: &mut VfsStatCursor = unsafe { ptr.as_mut() };
    match (cur.filetype, cur.field) {
        (FileType::Any, StatField::CurrentTime) => true as _,
        _ => false as _,
    }
}

#[no_mangle]
pub unsafe extern "C" fn VtabColumn(
    arg1: *mut sqlite3_vtab_cursor,
    ctx: *mut sqlite3_context,
    column: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let ptr = core::ptr::NonNull::new(arg1 as *mut VfsStatCursor).unwrap();
    let cur: &VfsStatCursor = unsafe { ptr.as_ref() };
    match column {
        0 => {
            // VSTAT_COLUMN_FILE
            unsafe {
                ((*crate::API).result_text.unwrap())(
                    ctx,
                    match cur.filetype {
                        FileType::Main => b"main\0".as_ptr() as _,
                        FileType::Journal => b"journal\0".as_ptr() as _,
                        FileType::Wal => b"wal\0".as_ptr() as _,
                        FileType::MasterJournal => b"master-journal\0".as_ptr() as _,
                        FileType::SubJournal => b"sub-journal\0".as_ptr() as _,
                        FileType::TempDb => b"temp-database\0".as_ptr() as _,
                        FileType::TempJournal => b"temp-journal\0".as_ptr() as _,
                        FileType::Transient => b"transient-db\0".as_ptr() as _,
                        FileType::Any => b"*\0".as_ptr() as _,
                    },
                    -1,
                    None,
                )
            };
        }
        1 => {
            // VSTAT_COLUMN_STAT
            unsafe {
                ((*crate::API).result_text.unwrap())(
                    ctx,
                    match cur.field {
                        StatField::BytesIn => b"bytesIn\0".as_ptr() as _,
                        StatField::BytesOut => b"bytesOut\0".as_ptr() as _,
                        StatField::Read => b"read\0".as_ptr() as _,
                        StatField::Write => b"write\0".as_ptr() as _,
                        StatField::Sync => b"sync\0".as_ptr() as _,
                        StatField::Open => b"open\0".as_ptr() as _,
                        StatField::Lock => b"lock\0".as_ptr() as _,
                        StatField::Access => b"access\0".as_ptr() as _,
                        StatField::Delete => b"delete\0".as_ptr() as _,
                        StatField::FullPath => b"fullPath\0".as_ptr() as _,
                        StatField::Random => b"random\0".as_ptr() as _,
                        StatField::Sleep => b"sleep\0".as_ptr() as _,
                        StatField::CurrentTime => b"currentTime\0".as_ptr() as _,
                    },
                    -1,
                    None,
                )
            };
        }
        2 => {
            //VSTAT_COLUMN_COUNT
            let vfs: *mut sqlite3_vfs =
                unsafe { ((*crate::API).vfs_find.unwrap())(crate::vfs::VFS_NAME.as_ptr() as _) };
            debug_assert!(!vfs.is_null());
            let vfs = core::ptr::NonNull::new(vfs).unwrap();
            let vfs_ptr: core::ptr::NonNull<crate::vfs::Vfs> =
                core::ptr::NonNull::new(unsafe { vfs.as_ref() }.pAppData as *mut crate::vfs::Vfs)
                    .expect("pAppData of stat vfs is null");
            let vfs_ = unsafe { vfs_ptr.as_ref() };
            unsafe {
                ((*crate::API).result_int64.unwrap())(
                    ctx,
                    match cur.field {
                        StatField::BytesIn => *statcnt!(vfs_.file_stats, cur.filetype, BytesIn),
                        StatField::BytesOut => *statcnt!(vfs_.file_stats, cur.filetype, BytesOut),
                        StatField::Read => *statcnt!(vfs_.file_stats, cur.filetype, Read),
                        StatField::Write => *statcnt!(vfs_.file_stats, cur.filetype, Write),
                        StatField::Sync => *statcnt!(vfs_.file_stats, cur.filetype, Sync),
                        StatField::Open => *statcnt!(vfs_.file_stats, cur.filetype, Open),
                        StatField::Lock => *statcnt!(vfs_.file_stats, cur.filetype, Lock),
                        StatField::Access => *statcnt!(vfs_.file_stats, cur.filetype, Access),
                        StatField::Delete => *statcnt!(vfs_.file_stats, cur.filetype, Delete),
                        StatField::FullPath => *statcnt!(vfs_.file_stats, cur.filetype, FullPath),
                        StatField::Random => *statcnt!(vfs_.file_stats, cur.filetype, Random),
                        StatField::Sleep => *statcnt!(vfs_.file_stats, cur.filetype, Sleep),
                        StatField::CurrentTime => {
                            *statcnt!(vfs_.file_stats, cur.filetype, CurrentTime)
                        }
                    } as i64,
                );
            }
        }
        _ => unreachable!("Unknown column number {}", column),
    }
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn VtabRowid(
    arg1: *mut sqlite3_vtab_cursor,
    pRowid: *mut sqlite3_int64,
) -> ::core::ffi::c_int {
    let ptr = core::ptr::NonNull::new(arg1 as *mut VfsStatCursor).unwrap();
    let cur: &VfsStatCursor = unsafe { ptr.as_ref() };
    let ftype_idx = cur.filetype as i64;
    let field_idx = cur.field as i64;
    unsafe {
        *pRowid = ftype_idx * 13 + field_idx;
    }
    SQLITE_OK as _
}

#[no_mangle]
pub unsafe extern "C" fn VtabUpdate(
    _arg1: *mut sqlite3_vtab,
    _arg2: ::core::ffi::c_int,
    _arg3: *mut *mut sqlite3_value,
    _arg4: *mut sqlite3_int64,
) -> ::core::ffi::c_int {
    SQLITE_ERROR as _
}

impl VTab {
    const VTAB_NAME: &'static [u8] = b"vtabstat\0";
    const VTAB_MODULE: sqlite3_module = sqlite3_module {
        iVersion: 0,
        xCreate: None,
        xConnect: Some(VtabConnect),
        xBestIndex: Some(VtabBestIndex),
        xDisconnect: Some(VtabDisconnect),
        xDestroy: None,
        xOpen: Some(VtabOpen),
        xClose: Some(VtabClose),
        xFilter: Some(VtabFilter),
        xNext: Some(VtabNext),
        xEof: Some(VtabEof),
        xColumn: Some(VtabColumn),
        xRowid: Some(VtabRowid),
        xUpdate: Some(VtabUpdate),
        xBegin: None,
        xSync: None,
        xCommit: None,
        xRollback: None,
        xFindFunction: None,
        xRename: None,
        xSavepoint: None,
        xRelease: None,
        xRollbackTo: None,
        xShadowName: None,
    };

    pub unsafe fn create(db: *mut sqlite3) -> Result<(), String> {
        let ret = unsafe {
            ((*crate::API).create_module.unwrap())(
                db,
                Self::VTAB_NAME.as_ptr() as _,
                &Self::VTAB_MODULE,
                core::ptr::null_mut(),
            )
        };
        if ret != SQLITE_OK as _ {
            return Err(format!("Could not create_module, returned {}", ret));
        }
        Ok(())
    }
}
