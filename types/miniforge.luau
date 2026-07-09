--!strict

export type Vector2 = {
    x: number,
    y: number,
}

export type Transform2D = {
    position: Vector2,
    rotation: number,
    scale: Vector2,
    size: Vector2,
}

export type Entity = {
    id: number,
    name: string,
    tag: string,
    layer: string,
    enabled: boolean,
    visible: boolean,
    transform: Transform2D,
}

export type Script = {
    entity: Entity,
    entity_id: number,
    entity_name: string,
    on_create: ((self: Script) -> ())?,
    on_ready: ((self: Script) -> ())?,
    on_update: ((self: Script, dt: number) -> ())?,
    on_fixed_update: ((self: Script, dt: number) -> ())?,
    on_collision_enter: ((self: Script, other: string) -> ())?,
    on_collision_exit: ((self: Script, other: string) -> ())?,
    on_destroy: ((self: Script) -> ())?,
}

declare Vector2: {
    new: (x: number?, y: number?) -> Vector2,
    length: (value: Vector2) -> number,
    normalized: (value: Vector2) -> Vector2,
}

declare Input: {
    is_pressed: (key: string) -> boolean,
    pressed: (key: string) -> boolean,
    get_axis: (negative: string, positive: string) -> number,
}

declare Time: {
    delta_time: number,
    fixed_delta_time: number,
    time: number,
    frame: number,
}

declare Entity: {
    current: () -> { id: number? },
    spawn: (name: string, x: number, y: number) -> (),
    destroy: (target: (number | string | Entity)?) -> (),
}

declare Transform2D: {
    set_position: (entity: Entity | number, x: number, y: number) -> (),
    translate: (entity: Entity | number, dx: number, dy: number) -> (),
}

declare Scene: {
    load: (name: string) -> (),
    current: () -> nil,
}

declare Events: {
    emit: (name: string, payload: any?) -> (),
}

declare Assets: {
    exists: (path: string) -> boolean,
    path: (path: string) -> string,
}

declare Debug: {
    log: (message: any) -> (),
    warn: (message: any) -> (),
    error: (message: any) -> (),
}

declare miniforge: {
    Vector2: typeof(Vector2),
    Input: typeof(Input),
    Time: typeof(Time),
    Entity: typeof(Entity),
    Transform2D: typeof(Transform2D),
    Scene: typeof(Scene),
    Events: typeof(Events),
    Assets: typeof(Assets),
    Debug: typeof(Debug),
}
