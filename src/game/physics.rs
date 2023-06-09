#![cfg(feature="server")]

use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::sync::{Mutex, OnceLock, MutexGuard};
use std::time::Duration;

static LOOPSTRUCT: OnceLock<PhysicsLoopStruct> = OnceLock::new();

/// cbindgen:ignore
#[allow(unused_doc_comments)]
extern "C" {
    // emscripten_set_main_loop_arg(em_arg_callback_func func, void *arg, int fps, int simulate_infinite_loop)
    #[cfg(all(target_family="wasm", target_os="emscripten"))]
    fn emscripten_set_main_loop(
        func: extern "C" fn() -> bool,
        fps: std::os::raw::c_int,
        simulate_infinite_loop: std::os::raw::c_int
    );
}

#[derive(Debug)]
#[repr(C)]
struct PhysicsLoopStruct {
    receive: Mutex<Receiver<()>>,  // Receive From Main Thread In Physics Thread
    i: AtomicI8
}

// This thread handles physics (aka the server)
pub fn start(tx: Sender<()>, rx: Receiver<()>) {
    run(rx).map_err(|err: String| {
        error!("Physics Crash: {:?}", err);
    }).ok();

    tx.send(()).ok();
}

fn run(rx: Receiver<()>) -> Result<(), String> {
    let loopstruct: PhysicsLoopStruct = PhysicsLoopStruct {
        receive: rx.into(), i: AtomicI8::new(0)
    };

    LOOPSTRUCT.set(loopstruct).unwrap();

    debug!("Starting Physics Loop...");
    /*
     * Intentionally not targetting feature "browser" here
     *   as emscripten is multi-platform.
     */
    #[cfg(all(target_family="wasm", target_os="emscripten"))]
    unsafe {
        emscripten_set_main_loop(physics_loop, 60, 1);
    }
    
    #[cfg(not(all(target_family="wasm", target_os="emscripten")))]
    loop {
        let exit_loop: bool = physics_loop();
        if exit_loop {
            // Ending Loop
            break;
        }

        // Slow Down Physics (60 FPS)
        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    debug!("Exiting Physics Loop...");

    Ok(())
}

fn should_terminate_thread(loopstruct: &PhysicsLoopStruct) -> bool {
    let rx: MutexGuard<Receiver<()>> = loopstruct.receive.lock().unwrap();

    match rx.try_recv() {
        Ok(_) => {
            return true;
        }
        Err(_) => {
            return false;
        }
    }
}

extern "C" fn physics_loop() -> bool {
    let loopstruct: &PhysicsLoopStruct = LOOPSTRUCT.get().unwrap();

    if should_terminate_thread(loopstruct) {
        debug!("Terminating Physics Thread...");
        return true;
    }
    
    loopstruct.i.fetch_add(1, Ordering::Relaxed);

    if loopstruct.i.load(Ordering::Relaxed) > 60 {
        loopstruct.i.store(0, Ordering::Relaxed);
    }

    update(loopstruct);

    return false;
}

fn update(_loopstruct: &PhysicsLoopStruct) {
    // debug!("Physics Update: {}", _loopstruct.i.load(Ordering::Relaxed));
}