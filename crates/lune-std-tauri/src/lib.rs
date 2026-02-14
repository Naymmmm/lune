use mlua::prelude::*;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener};

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

struct UnsafeLua(Lua);
unsafe impl Send for UnsafeLua {}
unsafe impl Sync for UnsafeLua {}

#[derive(Clone)]
struct LuaAppHandle(tauri::AppHandle);

impl LuaUserData for LuaAppHandle {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("emit", |_, this, (event, payload): (String, LuaValue)| {
            this.0
                .emit(&event, payload)
                .map_err(|e| LuaError::external(e))
        });
    }
}

/// TauriApp userdata - created by tauri.new()
#[derive(Clone)]
struct TauriApp {
    config: Arc<TauriConfig>,
    listeners: Arc<Mutex<Vec<(String, Arc<LuaRegistryKey>)>>>,
}

#[derive(Clone, Default)]
struct TauriConfig {
    name: String,
    identifier: String,
    version: String,
    icon: Option<String>,
    html: Option<String>,
    window_title: String,
    window_width: u32,
    window_height: u32,
}

impl LuaUserData for TauriApp {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // app:listen(event, callback)
        methods.add_method(
            "listen",
            |lua, this, (event, func): (String, LuaFunction)| {
                let mut list = this.listeners.lock().unwrap();
                list.push((event, Arc::new(lua.create_registry_value(func)?)));
                Ok(())
            },
        );

        // app:run()
        methods.add_method("run", |lua, this, ()| {
            let listeners = this.listeners.clone();
            let unsafe_lua = Arc::new(UnsafeLua(lua.clone()));

            let context = tauri::generate_context!("tauri.conf.json");

            tauri::Builder::default()
                .setup(move |app| {
                    let handle = app.handle();
                    let unsafe_lua = unsafe_lua.clone();
                    let list = listeners.lock().unwrap();

                    for (event_name, registry_key) in list.iter() {
                        let event_name = event_name.clone();
                        let registry_key = registry_key.clone();
                        let unsafe_lua = unsafe_lua.clone();
                        let app_handle = handle.clone();

                        handle.listen_any(event_name, move |event| {
                            let payload = event.payload().to_string();
                            let unsafe_lua = unsafe_lua.clone();
                            let registry_key = registry_key.clone();
                            let app_handle_inner = app_handle.clone();

                            let _ = app_handle.run_on_main_thread(move || {
                                let lua = &unsafe_lua.0;
                                if let Ok(func) = lua.registry_value::<LuaFunction>(&*registry_key)
                                {
                                    let lua_app = LuaAppHandle(app_handle_inner);
                                    let arg = if let Ok(val) =
                                        serde_json::from_str::<serde_json::Value>(&payload)
                                    {
                                        lua.to_value(&val).unwrap_or(LuaValue::Nil)
                                    } else {
                                        LuaValue::String(lua.create_string(&payload).unwrap())
                                    };

                                    let _ = func.call::<()>((arg, lua_app));
                                }
                            });
                        });
                    }

                    Ok(())
                })
                .run(context)
                .map_err(|e| LuaError::external(e))
        });
    }
}

pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;

    table.set("version", tauri::VERSION)?;

    // tauri.new(config) -> TauriApp
    table.set(
        "new",
        lua.create_function(|_, config: LuaTable| {
            let name = config
                .get::<String>("name")
                .unwrap_or_else(|_| "Lune App".to_string());
            let identifier = config
                .get::<String>("identifier")
                .unwrap_or_else(|_| "org.lune.app".to_string());
            let version = config
                .get::<String>("version")
                .unwrap_or_else(|_| "0.1.0".to_string());
            let icon = config.get::<String>("icon").ok();
            let html = config.get::<String>("html").ok();

            let (window_title, window_width, window_height) =
                if let Ok(window) = config.get::<LuaTable>("window") {
                    (
                        window
                            .get::<String>("title")
                            .unwrap_or_else(|_| name.clone()),
                        window.get::<u32>("width").unwrap_or(800),
                        window.get::<u32>("height").unwrap_or(600),
                    )
                } else {
                    (name.clone(), 800, 600)
                };

            Ok(TauriApp {
                config: Arc::new(TauriConfig {
                    name,
                    identifier,
                    version,
                    icon,
                    html,
                    window_title,
                    window_width,
                    window_height,
                }),
                listeners: Arc::new(Mutex::new(Vec::new())),
            })
        })?,
    )?;

    // Legacy: tauri.listen() and tauri.run() for backwards compatibility
    let listeners = Arc::new(Mutex::new(Vec::<(String, Arc<LuaRegistryKey>)>::new()));

    let listeners_clone = listeners.clone();
    table.set(
        "listen",
        lua.create_function(move |lua, (event, func): (String, LuaFunction)| {
            let mut list = listeners_clone.lock().unwrap();
            list.push((event, Arc::new(lua.create_registry_value(func)?)));
            Ok(())
        })?,
    )?;

    let unsafe_lua = Arc::new(UnsafeLua(lua.clone()));
    table.set(
        "run",
        lua.create_function(move |_, ()| {
            let listeners = listeners.clone();
            let unsafe_lua = unsafe_lua.clone();

            let context = tauri::generate_context!("tauri.conf.json");

            tauri::Builder::default()
                .setup(move |app| {
                    let handle = app.handle();
                    let unsafe_lua = unsafe_lua.clone();
                    let list = listeners.lock().unwrap();

                    for (event_name, registry_key) in list.iter() {
                        let event_name = event_name.clone();
                        let registry_key = registry_key.clone();
                        let unsafe_lua = unsafe_lua.clone();
                        let app_handle = handle.clone();

                        handle.listen_any(event_name, move |event| {
                            let payload = event.payload().to_string();
                            let unsafe_lua = unsafe_lua.clone();
                            let registry_key = registry_key.clone();
                            let app_handle_inner = app_handle.clone();

                            let _ = app_handle.run_on_main_thread(move || {
                                let lua = &unsafe_lua.0;
                                if let Ok(func) = lua.registry_value::<LuaFunction>(&*registry_key)
                                {
                                    let lua_app = LuaAppHandle(app_handle_inner);
                                    let arg = if let Ok(val) =
                                        serde_json::from_str::<serde_json::Value>(&payload)
                                    {
                                        lua.to_value(&val).unwrap_or(LuaValue::Nil)
                                    } else {
                                        LuaValue::String(lua.create_string(&payload).unwrap())
                                    };

                                    let _ = func.call::<()>((arg, lua_app));
                                }
                            });
                        });
                    }

                    Ok(())
                })
                .run(context)
                .map_err(|e| LuaError::external(e))
        })?,
    )?;

    Ok(table)
}

#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}
