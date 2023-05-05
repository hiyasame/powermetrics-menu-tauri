// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{process::{Command, Stdio}, cell::RefCell, sync::{Mutex, Arc}};
use tauri::{CustomMenuItem, SystemTrayMenuItem, SystemTrayEvent, AppHandle, Window, Manager, Icon};
use tokio::{process::Command as TokioCommand, io::AsyncWriteExt, io::{BufReader, AsyncBufReadExt}};

lazy_static::lazy_static! {
    static ref METRICS_PROCESS: Arc<Mutex<RefCell<Option<tokio::process::Child>>>> = Arc::new(Mutex::new(RefCell::new(None)));
}

#[tauri::command]
fn start_mertics(app: AppHandle, password: String, window: Window) {
    // 其实这个就是起一个协程，rs异步跟kt协程其实大同小异
    // js没有类似的东西是因为异步任务的调度全部交给了v8引擎，而rust将异步任务的调度交给了第三方库实现
    tokio::spawn(async move {
        println!("{}", password);
        // 起一个进程统计cpu功耗
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(format!("echo \"{}\" | sudo -S powermetrics", password));
        let process = TokioCommand::from(cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to start a process");
        // 将新的放入全局变量，拿出老的。如果有老的就杀掉
        // 杀完把锁释放掉
        {
            let old_process = METRICS_PROCESS
                .lock()
                .unwrap()
                .replace(Some(process));
            if let Some(mut p) = old_process {
                p.kill().await.unwrap();
            }
        }
        let mut process = METRICS_PROCESS.clone()
            .to_owned()
            .lock()
            .unwrap()
            .get_mut()
            .take()
            .unwrap();
        let stdout = process.stdout.take().expect("failed to open stdout");
        let mut reader = BufReader::new(stdout).lines();
        let mut first_time = true;
        loop {
            if let Some(line) = reader.next_line().await.unwrap() {
                println!("{}", line);
                // 这里拿到每行，把信息拿出来
                if line.starts_with("CPU Power:") {
                    let str = line.split(":").nth(1).unwrap().trim();
                    app.tray_handle()
                        .get_item("cpu")
                        .set_title(format!("CPU: {}", str))
                        .unwrap();
                } else if line.starts_with("GPU Power:") {
                    let str = line.split(":").nth(1).unwrap().trim();
                    app.tray_handle()
                        .get_item("gpu")
                        .set_title(format!("GPU: {}", str))
                        .unwrap();
                } else if line.starts_with("Combined Power (CPU + GPU + ANE):") {
                    let str = line.split(":").nth(1).unwrap().trim();
                    app.tray_handle().set_title(str).unwrap();
                } else if line.contains("Sorry") {
                    // 密码输错了
                    app.emit_all("wrong_password", true).unwrap();
                } else if first_time {
                    first_time = false;
                    // powermetrics 启动成功
                    app.emit_all("launch_success", true).unwrap();
                    window.hide().unwrap();
                }
            } else {
                break;
            }
        }
        process.wait().await.unwrap();
        
    });
}

#[tokio::main]
async fn main() {
    let context = tauri::generate_context!();
    let tray_menu = tauri::SystemTrayMenu::new()
                .add_item(CustomMenuItem::new("b1", "功耗").disabled())
                .add_item(CustomMenuItem::new("cpu", "CPU: 0 mW"))
                .add_item(CustomMenuItem::new("gpu", "GPU: 0 mW"))
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new("password", "输入密码").accelerator("p"))
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new("quit", "退出").accelerator("q"));
    let tray = tauri::SystemTray::new()
            // .with_icon(Icon::Rgba { rgba: vec![0, 0, 0, 0], width: 1, height: 1 })
            .with_icon_as_template(true)
            .with_title("PM")
            .with_menu(tray_menu)
            .with_menu_on_left_click(true);

    tauri::Builder::default()
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // don't kill the app when the user clicks close. this is important
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(|app| {
            // 不在 duck 上显示图标
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let window = app.get_window("main").unwrap();

            // this is a workaround for the window to always show in current workspace.
            // see https://github.com/tauri-apps/tauri/issues/2801
            window.set_always_on_top(true).unwrap();

            Ok(())
        })
        .system_tray(tray)
        .on_system_tray_event(menu_event_handler)
        .invoke_handler(tauri::generate_handler![start_mertics])
        .run(context)
        .expect("error while running tauri application");
}

fn menu_event_handler(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => {
            if id == "quit" {
                std::process::exit(0)
            } else if id == "password" {
                app.get_window("main").unwrap().show().unwrap();
            }
        }
        _ => {}
    }
}
