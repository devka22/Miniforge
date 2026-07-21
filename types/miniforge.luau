--!strict
-- MiniForge Luau runtime declarations. Keep this file aligned with
-- engine/luau_scripting.rs so editor diagnostics and completions match play mode.

export type Vector2 = {
    x: number,
    y: number,
}

export type Transform = {
    position: Vector2,
    rotation: number,
    scale: Vector2,
    size: Vector2,
}

export type Transform2D = Transform

export type EntityValue = {
    id: number,
    name: string,
    tag: string,
    layer: string,
    enabled: boolean,
    visible: boolean,
    transform: Transform,
    distance: number?,
}

export type Entity = EntityValue

export type EntityHandle = {
    id: number?,
    name: string?,
}

export type EntityTarget = number | string | EntityValue | EntityHandle

export type EntityQueryOptions = {
    tag: string?,
    tags: {string}?,
    layer: string?,
    layers: {string}?,
    include_disabled: boolean?,
    exclude_origin: boolean?,
}

export type PhysicsQueryOptions = {
    mask: (string | {string})?,
    layer: string?,
    layers: {string}?,
    include_triggers: boolean?,
    triggers: boolean?,
}

export type ShapeCastOptions = PhysicsQueryOptions & {
    shape: ("box" | "circle")?,
    radius: number?,
    half_extents: Vector2?,
}

export type RaycastHit = {
    id: number,
    entity_id: number,
    name: string,
    entity_name: string,
    point: Vector2,
    normal: Vector2,
    distance: number,
    layer: string,
    is_trigger: boolean,
}

export type CameraHandle = {
    follow: (self: CameraHandle, target: EntityTarget) -> (),
    shake: (self: CameraHandle, duration: number, amplitude: number) -> (),
    set_zoom: (self: CameraHandle, zoom: number) -> (),
    set_limits: (self: CameraHandle, min_x: number, min_y: number, max_x: number, max_y: number) -> (),
    pixel_perfect: (self: CameraHandle, enabled: boolean, pixels_per_unit: number?) -> (),
    world_to_screen: (self: CameraHandle, world: Vector2) -> Vector2,
    screen_to_world: (self: CameraHandle, screen: Vector2) -> Vector2,
}

export type TaskHandle = {
    remaining: number,
    cancelled: boolean,
}

export type Script = {
    entity: EntityValue,
    entity_id: number,
    entity_name: string,
    on_create: ((self: Script) -> ())?,
    on_start: ((self: Script) -> ())?,
    on_ready: ((self: Script) -> ())?,
    on_update: ((self: Script, dt: number) -> ())?,
    on_fixed_update: ((self: Script, dt: number) -> ())?,
    on_key_down: ((self: Script, key: string) -> ())?,
    on_collision_enter: ((self: Script, other: string) -> ())?,
    on_collision_exit: ((self: Script, other: string) -> ())?,
    on_event: ((self: Script, name: string, payload: any) -> ())?,
    on_destroy: ((self: Script) -> ())?,
}

declare global Vector2: {
    new: (x: number?, y: number?) -> Vector2,
    length: (value: Vector2) -> number,
    normalized: (value: Vector2) -> Vector2,
    add: (a: Vector2, b: Vector2) -> Vector2,
    sub: (a: Vector2, b: Vector2) -> Vector2,
    scale: (value: Vector2, factor: number) -> Vector2,
    dot: (a: Vector2, b: Vector2) -> number,
    distance: (a: Vector2, b: Vector2) -> number,
    lerp: (a: Vector2, b: Vector2, alpha: number) -> Vector2,
    move_towards: (current: Vector2, target: Vector2, max_delta: number) -> Vector2,
}

declare global Input: {
    is_pressed: (key: string) -> boolean,
    pressed: (key: string) -> boolean,
    get_axis: (negative: string, positive: string) -> number,
    action_pressed: (action: string) -> boolean,
    axis: (negative_action: string, positive_action: string) -> number,
}

declare global Time: {
    delta_time: number,
    fixed_delta_time: number,
    time: number,
    frame: number,
}

declare global Layers: {
    DEFAULT: string,
    PLAYER: string,
    PAWN: string,
    ENEMY: string,
    WORLD: string,
    WORLD_STATIC: string,
    TRIGGER: string,
    ONE_WAY_PLATFORM: string,
}

declare global Physics2D: {
    raycast: (origin: Vector2, target: Vector2, options: PhysicsQueryOptions?) -> RaycastHit?,
    shape_cast: (origin: Vector2, target: Vector2, options: ShapeCastOptions?) -> RaycastHit?,
    overlap_area: (center: Vector2, half_extents: Vector2, options: PhysicsQueryOptions?) -> {RaycastHit},
}

declare global Component: {
    add: (target: EntityTarget, component: string, data: {[string]: any}?) -> (),
    remove: (target: EntityTarget, component: string) -> (),
    set: (target: EntityTarget, component: string, key: string, value: any) -> (),
    get: (target: EntityTarget, component: string, key: string, default: any?) -> any,
    has: (target: EntityTarget, component: string) -> boolean,
}

declare global Rigidbody2D: {
    set_velocity: (target: EntityTarget, x: number, y: number) -> (),
    apply_impulse: (target: EntityTarget, x: number, y: number) -> (),
}

declare global CharacterBody2D: {
    move: (target: EntityTarget, x: number, y: number, jump: boolean?, run: boolean?) -> (),
}

declare global Camera: {
    main: () -> CameraHandle,
    current: () -> CameraHandle,
}

declare global AnimationPlayer: {
    play: ((animation: string) -> ()) & ((target: EntityTarget, animation: string) -> ()),
    set_parameter: (target: EntityTarget, key: string, value: any) -> (),
}

declare global AnimatedSprite: typeof(AnimationPlayer)

declare global Tilemap: {
    get_tile: (target: EntityTarget, layer: string, x: number, y: number) -> number?,
    set_tile: (target: EntityTarget, layer: string, x: number, y: number, tile: number) -> (),
}

declare global Tween: {
    to: (target: EntityTarget, property_path: string, to_value: number, duration: number, options: {easing: string?}?) -> (),
}

declare global Navigation2D: {
    set_destination: (target: EntityTarget, x: number, y: number) -> (),
}

declare global Audio2D: {
    play: (name: string, options: {bus: string?, volume: number?, loop: boolean?}?) -> (),
}

declare global Particles2D: {
    burst: (target: EntityTarget, count: number) -> (),
}

declare global Spawner: {
    spawn: (name: string, x: number, y: number) -> number,
}

declare global Entity: {
    current: () -> EntityHandle,
    spawn: (name: string, x: number, y: number, data: {[string]: any}?) -> number,
    find: (target: EntityTarget) -> EntityValue?,
    exists: (target: EntityTarget) -> boolean,
    count_with_tag: (tag: string) -> number,
    nearby: (origin: EntityTarget | Vector2, radius: number, options: EntityQueryOptions?) -> {EntityValue},
    nearest: (origin: EntityTarget | Vector2, radius: number, options: EntityQueryOptions?) -> EntityValue?,
    all_with_tag: (tag: string) -> {EntityValue},
    set_visible: (target: EntityTarget, visible: boolean) -> (),
    set_enabled: (target: EntityTarget, enabled: boolean) -> (),
    destroy: (target: EntityTarget?) -> (),
}

declare global Transform2D: {
    set_position: (entity: EntityTarget, x: number, y: number) -> (),
    translate: (entity: EntityTarget, dx: number, dy: number) -> (),
}

declare global Scene: {
    load: (name: string) -> (),
    current: () -> nil,
}

declare global Game: {
    save_slot: (slot: string) -> (),
    load_slot: (slot: string) -> (),
    autosave: () -> (),
}

declare global Events: {
    emit: (name: string, payload: any?) -> (),
}

declare global Assets: {
    exists: (path: string) -> boolean,
    path: (path: string) -> string,
}

declare global Debug: {
    log: (message: any) -> (),
    warn: (message: any) -> (),
    error: (message: any) -> (),
}

declare global Task: {
    delay: (seconds: number, callback: () -> ()) -> TaskHandle,
    defer: (callback: () -> ()) -> TaskHandle,
    cancel: (task: TaskHandle?) -> (),
}

declare global entity: EntityValue
declare global entity_id: number
declare global entity_name: string

-- Compatibility globals for concise scripts.
declare function move(dx: number, dy: number): ()
declare function set_position(x: number, y: number): ()
declare function spawn(name: string, x: number, y: number): ()
declare function destroy(target: EntityTarget?): ()
declare function load_scene(name: string): ()
declare function play_sound(name: string, bus: string?, volume: number?, looped: boolean?): ()
declare function input_pressed(key: string): boolean
declare function ui_text(text: string): ()
declare function set_ui_text(target: string, text: string): ()
declare function set_ui_progress(value: number, max: number): ()
declare function set_ui_progress_for(target: string, value: number, max: number): ()
declare function set_ui_visible(visible: boolean): ()
declare function set_ui_visible_for(target: string, visible: boolean): ()
declare function set_tag(tag: string): ()
declare function set_layer(layer: string): ()
declare function set_enabled(enabled: boolean): ()
declare function set_visible(visible: boolean): ()
declare function set_component_number(component: string, key: string, value: number): ()
declare function set_component_number_for(target: string, component: string, key: string, value: number): ()
declare function set_component_text(component: string, key: string, value: string): ()
declare function set_sprite(path: string): ()
declare function play_sprite_animation(frames_path: string, animation: string): ()
declare function set_sprite_flip(flip_x: boolean, flip_y: boolean): ()
declare function face_left(): ()
declare function face_right(): ()
declare function add_item(item_id: string, quantity: number): ()
declare function add_resource(resource_type: string, amount: number): ()
declare function add_quest(quest_id: string, title: string, objective_id: string?, objective_text: string?, target: number?): ()
declare function quest_progress(quest_id: string, objective_id: string, progress: any): ()
declare function complete_quest(quest_id: string): ()
declare function trigger_ability(now: number?): ()
declare function recharge_ability(amount: number): ()
declare function set_blackboard(key: string, value: any): ()

declare global miniforge: {
    Vector2: typeof(Vector2),
    Input: typeof(Input),
    Time: typeof(Time),
    Layers: typeof(Layers),
    Physics2D: typeof(Physics2D),
    Component: typeof(Component),
    Rigidbody2D: typeof(Rigidbody2D),
    CharacterBody2D: typeof(CharacterBody2D),
    Camera: typeof(Camera),
    AnimationPlayer: typeof(AnimationPlayer),
    AnimatedSprite: typeof(AnimatedSprite),
    Tilemap: typeof(Tilemap),
    Tween: typeof(Tween),
    Navigation2D: typeof(Navigation2D),
    Audio2D: typeof(Audio2D),
    Particles2D: typeof(Particles2D),
    Spawner: typeof(Spawner),
    Entity: typeof(Entity),
    Transform2D: typeof(Transform2D),
    Scene: typeof(Scene),
    Game: typeof(Game),
    Events: typeof(Events),
    Assets: typeof(Assets),
    Debug: typeof(Debug),
    move: typeof(move),
    set_position: typeof(set_position),
    spawn: typeof(spawn),
    destroy: typeof(destroy),
    load_scene: typeof(load_scene),
    play_sound: typeof(play_sound),
    input_pressed: typeof(input_pressed),
    ui_text: typeof(ui_text),
    set_ui_text: typeof(set_ui_text),
    set_ui_progress: typeof(set_ui_progress),
    set_ui_progress_for: typeof(set_ui_progress_for),
    set_ui_visible: typeof(set_ui_visible),
    set_ui_visible_for: typeof(set_ui_visible_for),
    set_tag: typeof(set_tag),
    set_layer: typeof(set_layer),
    set_enabled: typeof(set_enabled),
    set_visible: typeof(set_visible),
    set_component_number: typeof(set_component_number),
    set_component_number_for: typeof(set_component_number_for),
    set_component_text: typeof(set_component_text),
    set_sprite: typeof(set_sprite),
    play_sprite_animation: typeof(play_sprite_animation),
    set_sprite_flip: typeof(set_sprite_flip),
    face_left: typeof(face_left),
    face_right: typeof(face_right),
    add_item: typeof(add_item),
    add_resource: typeof(add_resource),
    add_quest: typeof(add_quest),
    quest_progress: typeof(quest_progress),
    complete_quest: typeof(complete_quest),
    trigger_ability: typeof(trigger_ability),
    recharge_ability: typeof(recharge_ability),
    set_blackboard: typeof(set_blackboard),
}
