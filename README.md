# Example sqlite3 Dynamic Loadable Extension in Rust - vfs and vtab modules

![Rust MSRV](https://img.shields.io/badge/rust_msrv-1.60.0-blue)

## The vfs and vtab

This is a port of the official [`ext/misc/vfsstat.c`](https://www.sqlite.org/src/file?name=ext/misc/vfsstat.c&ci=tip) sqlite3 extension. It implements a VFS and a virtual table that keeps IO statistics.

Query the virtual table by issuing `SELECT * FROM vtabstat`.

## Build

```shell
cargo build --release
```

Output will be located at `target/release/libvfsstat_rs.so`

*Note*: The crate includes trace and debug logs using the `log` crate but does not provide a `log` backend, so logs will not show up anywhere unless you set up a backend such as `env_logger` yourself.

## Use

Assuming `libvfsstat_rs.so` is in current directory,

```shell
$ sqlite3
sqlite> .load ./libvfsstat_rs
sqlite> .open ../../test.db
sqlite> .schema
CREATE TABLE person (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL UNIQUE,
                  data            BLOB
                  );
sqlite> select * from person;
id  name                              data
--  --------------------------------  -------------
1   Steven                            NULL
2   05504B041661AAD1320A8537EB8234D2  zÔ];[^;H
3   4967EEDBF69420D82C6B1BCFB31397DD  ?BiΘI
4   19D0F5974A48AA561B7097CDA39E4047
                                      l!Xk- 5
5   1F1FFD477777F5AA5CBFF916A4AF9593  + 텸vy\Y
6   0925676226D46FB4EC6DAEB11D130350  <</jޏ
7   5D6F4F219D2E3FAF7D61CA5237F106F8  (nlZ
8   FFD140BD25C1CA899446F2C9C2631A51  ,9^<      p
9   F9F66F44D64FC54FDFAEBD3750FD3513  [C@E
sqlite> select * from vtabstat;
file            stat         count
--------------  -----------  -----
main            bytesIn      12388
main            bytesOut     0
main            read         4
main            write        0
main            sync         0
main            open         1
main            lock         1
main            access       0
main            delete       0
main            fullPath     0
main            random       0
main            sleep        0
main            currentTime  0
journal         bytesIn      0
journal         bytesOut     0
journal         read         0
journal         write        0
journal         sync         0
journal         open         0
journal         lock         0
journal         access       0
journal         delete       0
journal         fullPath     0
journal         random       0
journal         sleep        0
journal         currentTime  0
wal             bytesIn      0
wal             bytesOut     0
wal             read         0
wal             write        0
wal             sync         0
wal             open         1
wal             lock         0
wal             access       0
wal             delete       0
wal             fullPath     0
wal             random       0
wal             sleep        0
wal             currentTime  0
master-journal  bytesIn      0
master-journal  bytesOut     0
master-journal  read         0
master-journal  write        0
master-journal  sync         0
master-journal  open         0
master-journal  lock         0
master-journal  access       0
master-journal  delete       0
master-journal  fullPath     0
master-journal  random       0
master-journal  sleep        0
master-journal  currentTime  0
sub-journal     bytesIn      0
sub-journal     bytesOut     0
sub-journal     read         0
sub-journal     write        0
sub-journal     sync         0
sub-journal     open         0
sub-journal     lock         0
sub-journal     access       0
sub-journal     delete       0
sub-journal     fullPath     0
sub-journal     random       0
sub-journal     sleep        0
sub-journal     currentTime  0
temp-database   bytesIn      0
temp-database   bytesOut     0
temp-database   read         0
temp-database   write        0
temp-database   sync         0
temp-database   open         0
temp-database   lock         0
temp-database   access       0
temp-database   delete       0
temp-database   fullPath     0
temp-database   random       0
temp-database   sleep        0
temp-database   currentTime  0
temp-journal    bytesIn      0
temp-journal    bytesOut     0
temp-journal    read         0
temp-journal    write        0
temp-journal    sync         0
temp-journal    open         0
temp-journal    lock         0
temp-journal    access       0
temp-journal    delete       0
temp-journal    fullPath     0
temp-journal    random       0
temp-journal    sleep        0
temp-journal    currentTime  0
transient-db    bytesIn      0
transient-db    bytesOut     0
transient-db    read         0
transient-db    write        0
transient-db    sync         0
transient-db    open         0
transient-db    lock         0
transient-db    access       0
transient-db    delete       0
transient-db    fullPath     0
transient-db    random       0
transient-db    sleep        0
transient-db    currentTime  0
*               bytesIn      0
*               bytesOut     0
*               read         0
*               write        0
*               sync         0
*               open         0
*               lock         0
*               access       2
*               delete       0
*               fullPath     1
*               random       0
*               sleep        0
```
