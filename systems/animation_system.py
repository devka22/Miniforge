class AnimationSystem:
    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = True
        self.run_in_play = True
        self.stats = {"animators": 0, "clips_sampled": 0}

    def update(self, dt):
        library = getattr(self.game, "animation_graphs", None)

        if not library:
            return

        animators = 0
        sampled = 0

        for entity in getattr(self.game.world, "entities", []):
            animator = entity.get_component("Animator") if hasattr(entity, "get_component") else None

            if not animator or not getattr(animator, "enabled", True):
                continue

            if self.game.mode == "EDITOR" and not getattr(animator, "preview", True):
                continue

            animators += 1
            controller = library.get(animator.controller)

            animator.parameters["moving"] = bool(getattr(entity, "path", [])) or getattr(entity, "state", "") == "MOVING"
            animator.current_state = controller.evaluate_transition(animator.current_state, animator.parameters)
            clip = controller.clip_for_state(animator.current_state)

            if not clip:
                continue

            animator.normalized_time += dt * max(0.0, float(animator.speed))
            frame = clip.sample(animator.normalized_time)
            sampled += 1
            self.apply_frame(entity, animator, frame)

        self.stats = {"animators": animators, "clips_sampled": sampled}

    def apply_frame(self, entity, animator, frame):
        sprite_renderer = entity.get_component("SpriteRenderer") if hasattr(entity, "get_component") else None
        transform = entity.get_component("Transform") if hasattr(entity, "get_component") else None

        if sprite_renderer and animator.apply_sprite and frame.get("sprite_name"):
            sprite_renderer.sprite_name = frame.get("sprite_name")
            entity.sprite_name = frame.get("sprite_name")

        if sprite_renderer and animator.apply_tint and frame.get("tint"):
            sprite_renderer.tint = tuple(frame.get("tint"))

        if transform:
            transform.scale_x = frame.get("scale_x", transform.scale_x)
            transform.scale_y = frame.get("scale_y", transform.scale_y)
