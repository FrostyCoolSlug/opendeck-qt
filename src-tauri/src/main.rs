// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application_watcher;
mod device_sleep;
mod elgato;
mod events;
mod plugins;
mod power_events;
mod qt;
mod shared;
mod store;
mod zip_extract;

mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use events::frontend;
use shared::PRODUCT_NAME;
use std::env;

use crate::plugins::SpawnRequest;
use crate::qt::backend::Backend;
use log::warn;
use qtbridge::{QApp, include_bytes_qml};
use std::sync::{OnceLock, mpsc};
use tauri::{
	AppHandle, Builder, Manager,
	menu::{IconMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
	tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
};


const MAIN_QML_TEMPLATE: &str = include_str!("../../static/Main.qml");

//static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static MESSAGE_BUS: OnceLock<MessageBus> = OnceLock::new();
struct MessageBus {
	pub tx: mpsc::Sender<SpawnRequest>,
}

fn show_window(app: &AppHandle) -> Result<(), tauri::Error> {
	#[cfg(target_os = "macos")]
	{
		use tauri::ActivationPolicy;
		let _ = app.set_activation_policy(ActivationPolicy::Regular);
	}

	let window = app.get_webview_window("main").ok_or_else(|| tauri::Error::WebviewNotFound)?;
	window.show().and_then(|_| window.set_focus())
}

fn hide_window(app: &AppHandle) -> Result<(), tauri::Error> {
	let window = app.get_webview_window("main").ok_or_else(|| tauri::Error::WebviewNotFound)?;
	window.hide()?;

	#[cfg(target_os = "macos")]
	{
		use tauri::ActivationPolicy;
		let _ = app.set_activation_policy(ActivationPolicy::Accessory);
	}

	Ok(())
}

#[tokio::main]
async fn main() {
	log_panics::init();
	let _ = fix_path_env::fix();

	#[cfg(target_os = "linux")]
	// SAFETY: std::env::set_var can cause race conditions in multithreaded contexts. We have not spawned any other threads at this point.
	unsafe {
		std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
		std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
	}

	// let app = match Builder::default()
	// 	.invoke_handler(tauri::generate_handler![
	// 		// restart,
	// 		// frontend::get_devices,
	// 		// frontend::get_port_base,
	// 		// frontend::get_categories,
	// 		// frontend::get_localisations,
	// 		// frontend::get_applications,
	// 		// frontend::get_application_profiles,
	// 		// frontend::set_application_profiles,
	// 		// frontend::get_fonts,
	// 		// frontend::instances::create_instance,
	// 		// frontend::instances::move_instance,
	// 		// frontend::instances::remove_instance,
	// 		// frontend::instances::set_state,
	// 		// frontend::instances::update_image,
	// 		// frontend::instances::trigger_virtual_press,
	// 		// frontend::profiles::get_profiles,
	// 		// frontend::profiles::get_selected_profile,
	// 		// frontend::profiles::set_selected_profile,
	// 		// frontend::profiles::delete_profile,
	// 		// frontend::profiles::rename_profile,
	// 		// frontend::property_inspector::make_info,
	// 		// frontend::property_inspector::switch_property_inspector,
	// 		// frontend::property_inspector::open_url,
	// 		// frontend::plugins::list_plugins,
	// 		// frontend::plugins::install_plugin,
	// 		// frontend::plugins::remove_plugin,
	// 		// frontend::plugins::reload_plugin,
	// 		// frontend::plugins::show_settings_interface,
	// 		// frontend::settings::get_settings,
	// 		// frontend::settings::set_settings,
	// 		// frontend::settings::open_config_directory,
	// 		// frontend::settings::open_log_directory,
	// 		// frontend::settings::get_build_info,
	// 		// frontend::settings::backup_config_directory,
	// 		// frontend::settings::restore_config_directory,
	// 	])
	// 	.setup(|app| {
	//
	// 		#[cfg(windows)]
	// 		if !std::env::args().any(|v| v == "--hide") {
	// 			let _ = app.get_webview_window("main").unwrap().show();
	// 		}
	// 		#[cfg(not(windows))]
	// 		if std::env::args().any(|v| v == "--hide") {
	// 			let _ = hide_window(app.handle());
	// 		}
	//
	// 		let old = app.path().config_dir().unwrap().join("com.amansprojects.opendeck");
	// 		if old.exists() {
	// 			let _ = std::fs::rename(old, app.path().app_config_dir().unwrap());
	// 		}
	//
	// 		let mut settings = store::get_settings();
	// 		use std::cmp::Ordering;
	// 		use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
	// 		let current_version = semver::Version::parse(built_info::PKG_VERSION)?;
	// 		let settings_version = semver::Version::parse(&settings.value.version)?;
	// 		let cmp = (current_version.major, current_version.minor).cmp(&(settings_version.major, settings_version.minor));
	//
	// 		plugins::initialise_plugins();
	// 		application_watcher::init_application_watcher();
	// 		device_sleep::init_device_sleep();
	// 		power_events::init_power_events();
	//
	// 		let label = IconMenuItemBuilder::with_id("label", PRODUCT_NAME)
	// 			.icon(app.default_window_icon().unwrap().clone())
	// 			.enabled(false)
	// 			.build(app)?;
	// 		let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
	// 		let hide = MenuItemBuilder::with_id("hide", "Hide").build(app)?;
	// 		let restart = MenuItemBuilder::with_id("restart", "Restart").build(app)?;
	// 		let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
	// 		let separator = PredefinedMenuItem::separator(app)?;
	// 		let menu = MenuBuilder::new(app).items(&[&label, &separator, &show, &hide, &separator, &restart, &quit]).build()?;
	// 		let _tray = TrayIconBuilder::with_id("opendeck")
	// 			.menu(&menu)
	// 			.icon(app.default_window_icon().unwrap().clone())
	// 			.show_menu_on_left_click(false)
	// 			.on_tray_icon_event(move |icon, event| {
	// 			})
	// 			.on_menu_event(move |app, event| {
	// 				let _ = match event.id().as_ref() {
	// 					"show" => show_window(app),
	// 					"hide" => hide_window(app),
	// 					"restart" => app.restart(),
	// 					"quit" => {
	// 						app.exit(0);
	// 						Ok(())
	// 					}
	// 					_ => Ok(()),
	// 				};
	// 			})
	// 			.build(app)?;
	//
	// 		#[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
	// 		{
	// 			use tauri_plugin_deep_link::DeepLinkExt;
	// 			let _ = app.deep_link().register_all();
	// 		}
	//
	// 		async fn update() -> Result<(), anyhow::Error> {
	// 			let res = reqwest::Client::new()
	// 				.get("https://api.github.com/repos/nekename/OpenDeck/releases/latest")
	// 				.header("Accept", "application/vnd.github+json")
	// 				.header("User-Agent", "OpenDeck")
	// 				.send()
	// 				.await?
	// 				.json::<serde_json::Value>()
	// 				.await?;
	// 			let tag_name = res.get("tag_name").unwrap().as_str().unwrap();
	// 			if semver::Version::parse(built_info::PKG_VERSION)?.cmp(&semver::Version::parse(&tag_name[1..])?) == Ordering::Less {
	// 			}
	//
	// 			Ok(())
	// 		}
	//
	// 		if settings.value.updatecheck {
	// 			tokio::spawn(async {
	// 				if let Err(error) = update().await {
	// 					log::warn!("Failed to update application: {error}");
	// 				}
	// 			});
	// 		}
	//
	// 		Ok(())
	// 	})
	// 	.build(tauri::generate_context!())
	// {
	// 	Ok(app) => app,
	// 	Err(error) => panic!("failed to build Tauri application: {}", error),
	// };

	// app.run(|app, event| {
	// 	if let tauri::RunEvent::Exit = event {
	// 		#[cfg(windows)]
	// 		futures::executor::block_on(plugins::deactivate_plugins());
	// 		tokio::spawn(elgato::reset_devices());
	// 		use tauri_plugin_aptabase::EventTracker;
	// 		app.flush_events_blocking();
	// 	}
	// });

	let mut flags = vec![
		"--disable-software-rasterizer",
		"--disable-dev-shm-usage",
		"--disable-background-networking",
		"--disable-renderer-backgrounding",
		"--js-flags=--expose-gc,--max-old-space-size=192",
		"--num-raster-threads=2",
		"--single-process",
		"--disable-web-security",
	];

	// Disable the Chromium Sandbox when running from a flatpak, as flatpak handles
	// the sandboxing, and this can conflict
	if env::var("FLATPAK_SANDBOX_DIR").is_ok() {
		flags.push("--no-sandbox");
		flags.push("--disable-setuid-sandbox");
	}

	unsafe {
		//env::set_var("QT_QPA_PLATFORM", "xcb");
		env::set_var("RUST_LOG", "info");
		env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", flags.join(" "));
	}

	env_logger::init();

	plugins::initialise_plugins();
	application_watcher::init_application_watcher();
	device_sleep::init_device_sleep();
	power_events::init_power_events();

	//embedded_qt_resources();

	tokio::spawn(async {
		loop {
			elgato::initialise_devices().await;
			tokio::time::sleep(std::time::Duration::from_secs(10)).await;
		}
	});

	//include_bytes_qml!("../../build/index.html", "qt/qml/App");

	let port = start_asset_server();
	let web_root = format!("http://127.0.0.1:{port}/");

	let qml = MAIN_QML_TEMPLATE.replace("__WEB_ROOT__", &web_root);

	QApp::new()
		.register::<Backend>()
		.load_qml(qml.as_bytes())
		.run();

	#[cfg(windows)]
	futures::executor::block_on(plugins::deactivate_plugins());

	tokio::spawn(elgato::reset_devices());
}

use include_dir::{include_dir, Dir};
use tiny_http::{Server, Response, Header};
use std::{net::TcpListener, thread};

// $CARGO_MANIFEST_DIR interpolation is a documented include_dir feature —
// this is resolved at compile time, relative to the crate root, no build.rs needed.
static WEB_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../build");

fn start_asset_server() -> u16 {
	let listener = TcpListener::bind("127.0.0.1:0").unwrap();
	let port = listener.local_addr().unwrap().port();
	let server = Server::from_listener(listener, None).unwrap();

	thread::spawn(move || {
		for request in server.incoming_requests() {
			let path = request.url().trim_start_matches('/');
			let path = if path.is_empty() { "index.html" } else { path };

			// Fall back to index.html for client-side routing (SvelteKit etc.)
			let file = WEB_DIR.get_file(path).or_else(|| WEB_DIR.get_file("index.html"));

			let response = match file {
				Some(f) => {
					let mime = mime_guess::from_path(f.path()).first_or_octet_stream();
					Response::from_data(f.contents())
						.with_header(Header::from_bytes(&b"Content-Type"[..], mime.essence_str().as_bytes()).unwrap())
				}
				None => Response::from_data(&b"Not Found"[..][..]).with_status_code(404),
			};

			let _ = request.respond(response);
		}
	});

	port
}

command_macros::generate_handler![
	frontend::restart,
	frontend::get_devices,
	frontend::get_port_base,
	frontend::get_categories,
	frontend::get_localisations,
	frontend::get_applications,
	frontend::get_application_profiles,
	frontend::set_application_profiles,
	frontend::get_fonts,
	frontend::instances::create_instance,
	frontend::instances::move_instance,
	frontend::instances::remove_instance,
	frontend::instances::set_state,
	frontend::instances::update_image,
	frontend::instances::trigger_virtual_press,
	frontend::profiles::get_profiles,
	frontend::profiles::get_selected_profile,
	frontend::profiles::set_selected_profile,
	frontend::profiles::delete_profile,
	frontend::profiles::rename_profile,
	frontend::property_inspector::make_info,
	frontend::property_inspector::switch_property_inspector,
	frontend::property_inspector::open_url,
	frontend::plugins::list_plugins,
	frontend::plugins::install_plugin,
	frontend::plugins::remove_plugin,
	frontend::plugins::reload_plugin,
	frontend::plugins::show_settings_interface,
	frontend::settings::get_settings,
	frontend::settings::set_settings,
	frontend::settings::open_config_directory,
	frontend::settings::open_log_directory,
	frontend::settings::get_build_info,
	frontend::settings::backup_config_directory,
	frontend::settings::restore_config_directory,
];
