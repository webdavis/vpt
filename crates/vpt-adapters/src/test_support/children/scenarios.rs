use crate::RootDir;
use std::io::{self, Write};
use std::path::Path;

pub(crate) fn ready() {
    println!("\nVPT_READY");
    io::stdout().flush().unwrap();
    let mut command = String::new();
    io::stdin().read_line(&mut command).unwrap();
    assert_eq!(command, "continue\n");
}

pub(crate) fn native(path: &Path) -> rusqlite::Connection {
    assert!(
        // SAFETY: SQLite owns the registration and the name is a static C string.
        !unsafe { rusqlite::ffi::sqlite3_vfs_find(c"unix-posix".as_ptr()) }.is_null(),
        "bundled SQLite must register the POSIX record-lock reference filesystem"
    );
    let connection = rusqlite::Connection::open_with_flags_and_vfs(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        "unix-posix",
    )
    .unwrap();
    connection.busy_timeout(std::time::Duration::ZERO).unwrap();
    connection
}

#[test]
fn child() {
    let Ok(scenario) = std::env::var("VPT_TEST_SCENARIO") else {
        return;
    };
    if let Some(expected) = std::env::var_os("VPT_TEST_ROOT_PID") {
        // SAFETY: getppid only observes the parent process identifier.
        let parent = unsafe { libc::getppid() };
        assert_eq!(
            parent.to_string(),
            expected
                .into_string()
                .expect("root test process identifier"),
            "test participant must be a direct child of the root test"
        );
    }
    let directory = std::env::var_os("VPT_TEST_ROOT").expect("owned root");
    let opening = crate::ledger::schema_diagnostics::start("root.open.start");
    let root = RootDir::open(&Path::new(&directory).join("state")).unwrap();
    crate::ledger::schema_diagnostics::end(opening, "root.open.end");
    match scenario.as_str() {
        "ledger_contract_history_budget_exit"
        | "ledger_contract_history_budget_stall"
        | "budget_exit" => super::budget::finite_child(&scenario, Path::new(&directory)),
        "identity_exit" | "identity_signal" | "identity_grace" => {
            super::identity::finite_child(&scenario, Path::new(&directory))
        }
        "supervision_ready" => ready(),
        "supervision_twice" => {
            ready();
            ready();
        }
        "supervision_stall" => {
            ready();
            eprintln!("VPT_PHASE pre-sql-stall");
            loop {
                std::thread::park();
            }
        }
        s if s.starts_with("ledger_") => crate::ledger::run_scenario(s, root, ready, native),
        s if s.starts_with("lock_") => crate::lock::tests::scenarios::run(s, root, ready),
        s if s.starts_with("recorder_") => crate::voice_memos::run_scenario(s, root),
        s if s.starts_with("title_") => crate::voice_memos::run_title_scenario(s, root, ready),
        "stall" => {
            // SAFETY: this supervised fixture ignores termination to exercise forced cleanup.
            unsafe {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
            }
            println!("OUT\t\u{1b}{}", "o".repeat(1024));
            io::stdout().flush().unwrap();
            eprintln!(
                "ERR\t\u{1b}{}\nVPT_PHASE stall-start\nVPT_PHASE stall-wait",
                "e".repeat(1024)
            );
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            panic!("stall fixture unexpectedly received input");
        }
        _ => crate::database::tests::run_scenario(&scenario, root),
    }
}
