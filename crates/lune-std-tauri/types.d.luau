export type WindowConfig = {
    title: string?,
    width: number?,
    height: number?,
}

export type TauriConfig = {
    name: string?,
    identifier: string?,
    version: string?,
    icon: string?,
    html: string?,
    window: WindowConfig?,
}

export type AppHandle = {
    emit: (event: string, payload: any) -> (),
}

export type TauriApp = {
    listen: (event: string, callback: (payload: any, app: AppHandle) -> ()) -> (),
    run: () -> (),
}

export type Tauri = {
    version: string,
    new: (config: TauriConfig?) -> TauriApp,
    -- Legacy API
    listen: (event: string, callback: (payload: any, app: AppHandle) -> ()) -> (),
    run: () -> (),
}

return nil :: Tauri
