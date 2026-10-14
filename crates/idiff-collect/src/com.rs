//! COM on the current thread, for as long as a source's read lasts. Each source runs on its own
//! thread, so a source that uses COM or WinRT makes one of these first.
//!
//! WinRT calls on a thread that never initialized COM borrow the process's shared multithreaded
//! apartment, which ends when the last thread that joined it leaves. With sources reading in
//! parallel, that was the Task Scheduler thread finishing while Store apps were still being
//! read, and the process crashed. So every thread joins explicitly.

use windows::Win32::System::Com::{
    COINIT, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
};

pub struct Com(bool);

impl Com {
    /// Single-threaded, which the shell's objects (shortcuts) expect.
    pub fn sta() -> Com {
        Com::init(COINIT_APARTMENTTHREADED)
    }

    /// The shared multithreaded apartment, for Task Scheduler and WinRT.
    pub fn mta() -> Com {
        Com::init(COINIT_MULTITHREADED)
    }

    fn init(mode: COINIT) -> Com {
        // If the thread already has COM in another mode, it still works; just don't release it.
        Com(unsafe { CoInitializeEx(None, mode) }.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}
