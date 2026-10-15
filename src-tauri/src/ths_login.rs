//! 同花顺免密登录：内嵌官方登录页（短信验证码 / App 扫码），登录后提取 WebView cookie 完成同步。
use crate::models::StateSnapshot;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_2;
use webview2_com::{CoTaskMemPWSTR, GetCookiesCompletedHandler};
use windows_core::{Interface, PCWSTR};

pub const LOGIN_WINDOW_LABEL: &str = "ths-login";
/// 登录成功后官方页会 302 到 redir 指定的自选股页面，以此作为登录成功的信号
const LOGIN_URL: &str = "https://upass.10jqka.com.cn/login?redir=https%3A%2F%2Fstock.10jqka.com.cn%2Fmy%2Fzixuan.shtml";

/// 打开同花顺官方登录窗口（短信验证码 / 扫码 / 密码均可）
#[tauri::command]
pub fn open_ths_login(app: AppHandle) -> Result<(), String> {
    log(&app, "open_ths_login 被调用".into());
    if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        // 残留窗口：强制显示并重新导航到登录页
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.eval(&format!("location.href='{LOGIN_URL}'"));
        log(&app, format!("复用已有窗口 show+导航, visible={:?}", win.is_visible()));
        return Ok(());
    }
    // 窗口创建放到独立线程：在 command/IPC 线程里同步调用 build() 会与
    // 事件循环死锁，导致 WebView 初始化不完整（白屏、页面不加载）。
    std::thread::Builder::new()
        .name("ths-login-win".into())
        .spawn(move || create_login_window(app))
        .map_err(|e| format!("启动失败: {e}"))?;
    Ok(())
}

fn create_login_window(app: AppHandle) {
    let url: tauri::Url = match LOGIN_URL.parse() {
        Ok(u) => u,
        Err(e) => {
            log(&app, format!("登录地址解析失败: {e}"));
            return;
        }
    };
    let app_for_load = app.clone();
    let built = WebviewWindowBuilder::new(&app, LOGIN_WINDOW_LABEL, WebviewUrl::External(url))
        .title("同花顺登录 · TICK")
        .inner_size(980.0, 700.0)
        .center()
        .on_page_load(move |_, payload| {
            log(&app_for_load, format!("page_load: {:?}", payload.event()));
        })
        .build();
    match &built {
        Ok(_) => log(&app, "窗口创建成功".into()),
        Err(e) => {
            log(&app, format!("窗口创建失败: {e}"));
            return;
        }
    }

    // 登录成功检测：轮询窗口 URL。
    // 不用 builder 的 on_navigation——Windows 上它会导致窗口创建静默失败。
    let app = Arc::new(app);
    tauri::async_runtime::spawn(async move {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        while std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            // 用户手动关掉登录窗口则停止监控
            let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) else { break };
            let Ok(cur) = win.url() else { continue };
            if cur.host_str() == Some("stock.10jqka.com.cn") && cur.path().contains("zixuan") {
                log(&app, format!("检测到登录成功跳转: {cur}"));
                // 等页面加载、cookie 落定
                tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                match confirm_login(&app).await {
                    Ok(_) => break,
                    Err(e) => log(&app, format!("自动同步失败: {e:#}")),
                }
            }
        }
    });
}

/// 简易文件日志（GUI 程序无控制台，排查"没反应"类问题用）
pub fn log(app: &AppHandle, msg: String) {
    use std::io::Write;
    let Ok(dir) = app.path().app_data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("stocker.log"))
    {
        let _ = writeln!(f, "[{}] {msg}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    }
}

/// 手动确认登录（兜底：自动检测没触发时由前端按钮调用）
#[tauri::command]
pub async fn confirm_ths_login(app: AppHandle) -> Result<StateSnapshot, String> {
    confirm_login(&app).await.map_err(|e| format!("{e:#}"))
}

/// 提取登录窗口 cookie → 验证并持久化 → 关窗 → 通知前端
pub async fn confirm_login(app: &AppHandle) -> Result<StateSnapshot> {
    let cookie = extract_cookies(app)?;
    let has_userid = cookie.split("; ").any(|kv| kv.starts_with("userid="));
    if !has_userid {
        anyhow::bail!("未检测到登录态：请在登录窗口完成登录（短信验证码或手机 App 扫码）后重试");
    }
    let state = app.state::<Arc<AppState>>();
    let snapshot = crate::commands::apply_cookie(&state, cookie)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = win.close();
    }
    let _ = app.emit("ths-login-done", "登录成功，已同步同花顺自选分组");
    Ok(snapshot)
}

/// 打开官方找回密码页（浏览器）
#[tauri::command]
pub fn open_ths_forgot() -> Result<(), String> {
    tauri_plugin_opener::open_url("https://upass.10jqka.com.cn/lostpass", None::<&str>)
        .map_err(|e| format!("打开页面失败: {e}"))
}

/// 从登录窗口提取全部 10jqka 域 cookie，拼成 "k=v; k2=v2"
fn extract_cookies(app: &AppHandle) -> Result<String> {
    let win = app
        .get_webview_window(LOGIN_WINDOW_LABEL)
        .ok_or_else(|| anyhow!("登录窗口未打开，请先点击「打开登录窗口」"))?;
    let (tx, rx) = std::sync::mpsc::channel::<Result<String>>();
    win.with_webview(move |wv| {
        let _ = tx.send(grab_cookies(wv));
    })
    .map_err(|e| anyhow!("访问登录窗口失败: {e}"))?;
    rx.recv_timeout(std::time::Duration::from_secs(10))?
}

/// 主线程内执行：CookieManager 拉全部 cookie（含 httpOnly），过滤 10jqka 域
fn grab_cookies(wv: tauri::webview::PlatformWebview) -> Result<String> {
    unsafe {
        let controller = wv.controller();
        let core = controller.CoreWebView2()?;
        let core2: ICoreWebView2_2 = core.cast()?;
        let manager = core2.CookieManager()?;

        let (tx, rx) = std::sync::mpsc::channel::<windows_core::Result<Vec<(String, String, String)>>>();
        manager.GetCookies(
            PCWSTR::null(),
            &GetCookiesCompletedHandler::create(Box::new(move |error_code, list| {
                let result = (|| {
                    error_code?;
                    let list = list.ok_or_else(|| {
                        windows_core::Error::from(windows_core::HRESULT(-2147467259))
                    })?;
                    let mut count = 0u32;
                    list.Count(&mut count)?;
                    let mut out = Vec::with_capacity(count as usize);
                    for i in 0..count {
                        let c = list.GetValueAtIndex(i)?;
                        let mut name = windows_core::PWSTR::null();
                        let mut value = windows_core::PWSTR::null();
                        let mut domain = windows_core::PWSTR::null();
                        c.Name(&mut name)?;
                        c.Value(&mut value)?;
                        c.Domain(&mut domain)?;
                        // 接管所有权，drop 时自动 CoTaskMemFree
                        let (d, n, v) = (
                            CoTaskMemPWSTR::from(domain).to_string(),
                            CoTaskMemPWSTR::from(name).to_string(),
                            CoTaskMemPWSTR::from(value).to_string(),
                        );
                        out.push((d, n, v));
                    }
                    Ok(out)
                })();
                tx.send(result).map_err(|_| {
                    windows_core::Error::from(windows_core::HRESULT(-2147467259))
                })
            })),
        )?;

        let cookies: Vec<(String, String, String)> = webview2_com::wait_with_pump(rx)??;
        // 主域 cookie 优先，同名去重
        let mut ordered: Vec<(String, String, String)> =
            cookies.iter().filter(|(d, _, _)| d == ".10jqka.com.cn").cloned().collect();
        ordered.extend(cookies.iter().filter(|(d, _, _)| d.ends_with("10jqka.com.cn") && d != ".10jqka.com.cn").cloned());
        let mut seen = HashSet::new();
        let mut parts: Vec<String> = Vec::new();
        for (_, name, value) in ordered {
            if name.is_empty() || !seen.insert(name.clone()) {
                continue;
            }
            parts.push(format!("{name}={value}"));
        }
        Ok(parts.join("; "))
    }
}
