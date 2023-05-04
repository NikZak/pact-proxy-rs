#[cfg(feature = "flame_it")]
use flame as f;
#[cfg(feature = "flame_it")]
use flamer::flame;
#[cfg(test)]
use signal_hook::{
    consts::signal::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::error::Error;
#[cfg(test)]
use std::io::Write;
use std::path::Path;
#[cfg(test)]
use std::{io, thread};

pub fn create_folder_if_not_exists(folder_path: &Path) -> Result<(), Box<dyn Error>> {
    if !folder_path.exists() {
        std::fs::create_dir_all(folder_path)?;
    }
    Ok(())
}

#[cfg(test)]
pub fn set_hook_on_panic_or_signal<F: Fn() + Sync + Send + 'static + Clone>(
    hook: F,
) -> Result<(), Box<dyn Error>> {
    let hook_cloned = hook.clone();
    thread::spawn(move || {
        let mut signals = Signals::new([SIGINT, SIGTERM])?;
        for sig in signals.forever() {
            hook_cloned.call(());
            // flush stdout and stderr of not only this thread but all threads in the process
            io::stdout().flush().unwrap();
            io::stderr().flush().unwrap();

            std::process::exit(sig);
        }
        Ok::<(), io::Error>(())
    });
    std::panic::set_hook(Box::new(move |panic_info| {
        hook.call(());
        // resume panic
        println!("Panic: {panic_info}");
    }));

    Ok(())
}

#[cfg(feature = "flame_it")]
fn dump_flame_file(url: &str) {
    let file_name = "flamegraph-".to_string() + url.replace("/", "_").as_str() + ".html";
    f::dump_html(File::create(file_name).unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_set_hook_on_panic() {
        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_called_cloned = hook_called.clone();
        set_hook_on_panic_or_signal(move || {
            println!("Hook called");
            hook_called_cloned.store(true, Ordering::SeqCst);
        })
        .unwrap();
        let _ = std::panic::catch_unwind(|| {
            panic!("Panic");
        });
        assert!(hook_called.load(Ordering::SeqCst));
    }
}
