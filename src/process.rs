use anyhow::{Result, bail, ensure};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WAIT_TIMEOUT},
    System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    },
};

fn title(hwnd: HWND) -> String {
    let mut text = [0u16; 512];
    let length = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
    String::from_utf16_lossy(&text[..length.max(0) as usize])
}
fn pid(hwnd: HWND) -> u32 {
    let mut result = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut result);
    }
    result
}
fn unique_process(mut ids: Vec<u32>) -> Result<u32> {
    ids.sort_unstable();
    ids.dedup();
    match ids.as_slice() {
        [id] => Ok(*id),
        [] => bail!(
            "Game window not found. Open WoW first, or select --process-id PID / --window TITLE."
        ),
        _ => bail!("Multiple matching game processes {ids:?}. Select one with --process-id PID."),
    }
}
pub fn find_process(expected: &str) -> Result<u32> {
    struct Search {
        title: String,
        ids: Vec<u32>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        // EnumWindows invokes this synchronously while Search is alive.
        let search = unsafe { &mut *(data as *mut Search) };
        if title(hwnd).to_lowercase().contains(&search.title) {
            let id = pid(hwnd);
            if id != 0 {
                search.ids.push(id);
            }
        }
        1
    }
    let mut search = Search {
        title: expected.to_lowercase(),
        ids: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(visit), (&mut search as *mut Search) as LPARAM);
    }
    unique_process(search.ids)
}
pub fn foreground_matches(expected: &str, process_id: Option<u32>) -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    process_id.is_none_or(|id| pid(hwnd) == id)
        && title(hwnd)
            .to_lowercase()
            .contains(&expected.to_lowercase())
}
/// Holds process identity after exit to prevent PID reuse.
pub struct ProcessGuard(HANDLE);
impl ProcessGuard {
    pub fn open(id: u32) -> Result<Self> {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, id) };
        ensure!(
            !handle.is_null(),
            "Cannot open selected process {id}: {}",
            std::io::Error::last_os_error()
        );
        let guard = Self(handle);
        ensure!(guard.running(), "Selected process has already exited");
        Ok(guard)
    }
    pub fn running(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, 0) == WAIT_TIMEOUT }
    }
}
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_requires_one_process_but_allows_multiple_windows() {
        assert_eq!(unique_process(vec![42, 42]).unwrap(), 42);
        assert!(unique_process(vec![]).is_err());
        assert!(unique_process(vec![42, 43]).is_err());
    }
}
