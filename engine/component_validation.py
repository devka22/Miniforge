class ComponentValidation:
    NUMERIC_RANGES = {
        "RTSMovement.speed": (0.0, 100.0),
        "RTSMovement.acceleration": (0.0, 100.0),
        "RTSMovement.turn_speed": (0.0, 100.0),
        "AudioSource.volume": (0.0, 1.0),
        "AudioSource.pitch": (0.1, 4.0),
        "AudioSource.spatial_blend": (0.0, 1.0),
        "Rigidbody2D.mass": (0.0001, 100000.0),
        "Rigidbody2D.drag": (0.0, 100.0),
        "Rigidbody2D.angular_drag": (0.0, 100.0),
        "Rigidbody2D.bounciness": (0.0, 1.0),
        "Rigidbody2D.friction": (0.0, 1.0),
        "Collider2D.width": (0.01, 1000.0),
        "Collider2D.height": (0.01, 1000.0),
        "Collider2D.radius": (0.01, 1000.0),
        "Animator.speed": (0.0, 100.0),
        "UIElement.width": (1.0, 10000.0),
        "UIElement.height": (1.0, 10000.0),
        "UIElement.opacity": (0.0, 1.0),
        "UIElement.padding": (0.0, 500.0),
        "UIElement.border_radius": (0.0, 128.0),
        "UIElement.font_size": (0.0, 256.0),
        "UIElement.progress": (0.0, 1000000000.0),
        "UIElement.max_progress": (0.0001, 1000000000.0),
        "Stats.level": (1.0, 100000.0),
        "Stats.strength": (0.0, 100000.0),
        "Stats.agility": (0.0, 100000.0),
        "Stats.intelligence": (0.0, 100000.0),
        "Stats.vitality": (0.0, 100000.0),
        "Stats.attack": (0.0, 1000000.0),
        "Stats.defense": (0.0, 1000000.0),
        "Stats.critical_chance": (0.0, 1.0),
        "Stats.critical_multiplier": (1.0, 100.0),
        "Inventory.capacity": (0.0, 10000.0),
        "Inventory.stack_limit": (1.0, 100000.0),
        "Ability.cooldown": (0.0, 100000.0),
        "Ability.range": (0.0, 100000.0),
        "Ability.power": (0.0, 1000000.0),
        "Ability.charges": (0.0, 10000.0),
        "AIController.think_interval": (0.02, 60.0),
        "AIController.detection_radius": (0.0, 100000.0),
        "AIController.attack_radius": (0.0, 100000.0),
        "AIController.leash_radius": (0.0, 100000.0),
        "NavAgent.speed": (0.0, 1000.0),
        "NavAgent.stopping_distance": (0.0, 1000.0),
        "NavAgent.repath_interval": (0.02, 60.0),
        "Interaction.radius": (0.0, 100000.0),
        "Lifetime.duration": (-1.0, 1000000.0),
        "Spawner.spawn_interval": (0.0, 100000.0),
        "Spawner.spawn_radius": (0.0, 100000.0),
        "Spawner.max_alive": (0.0, 100000.0),
        "DamageDealer.damage": (0.0, 1000000.0),
        "DamageDealer.cooldown": (0.0, 100000.0),
        "CameraFollow.smoothness": (0.0, 1000.0),
        "CameraFollow.zoom": (0.01, 100.0),
        "Timer.duration": (0.0, 1000000.0),
        "Tween.duration": (0.0, 1000000.0),
        "Light2D.radius": (0.0, 100000.0),
        "Light2D.intensity": (0.0, 1000.0),
        "ParallaxLayer.factor_x": (-100.0, 100.0),
        "ParallaxLayer.factor_y": (-100.0, 100.0),
        "CharacterController2D.walk_speed": (0.0, 1000.0),
        "CharacterController2D.jump_force": (0.0, 1000.0),
        "EconomyWallet.capacity": (0.0, 1000000000.0),
    }

    VALID_VALUES = {
        "Rigidbody2D.body_type": {"dynamic", "kinematic", "static"},
        "Collider2D.shape": {"rect", "circle"},
        "UIElement.element_type": {"Label", "Button", "Image", "Panel", "ProgressBar"},
        "UIElement.text_align": {"left", "center", "right"},
        "UIElement.anchor": {
            "top_left",
            "top_right",
            "bottom_left",
            "bottom_right",
            "center",
            "right",
            "bottom",
            "stretch_width",
            "stretch_height",
            "stretch",
        },
        "Ability.target_mode": {"self", "entity", "point", "area", "none"},
        "AIController.behavior": {"idle", "wander", "chase", "attack", "guard"},
        "AIController.state": {"idle", "wander", "chase", "attack", "return"},
        "DamageDealer.damage_type": {"physical", "fire", "ice", "magic", "true"},
        "Tween.easing": {"linear", "smooth", "ease_in", "ease_out"},
    }

    @classmethod
    def validate_component(cls, component):
        warnings = []
        errors = []
        component_type = getattr(component, "component_type", None)

        if not component_type:
            return warnings, ["Componente sin component_type"]

        for attr, value in vars(component).items():
            path = f"{component_type}.{attr}"

            if path in cls.NUMERIC_RANGES:
                minimum, maximum = cls.NUMERIC_RANGES[path]

                try:
                    numeric = float(value)
                except Exception:
                    errors.append(f"{path} debe ser numérico")
                    continue

                if numeric < minimum or numeric > maximum:
                    warnings.append(f"{path} fuera de rango [{minimum}, {maximum}]: {value}")

            if path in cls.VALID_VALUES and value not in cls.VALID_VALUES[path]:
                warnings.append(f"{path} inválido: {value}")

        if component_type == "VisualScript":
            warnings.extend(cls.validate_visual_script(component))

        return warnings, errors

    @classmethod
    def repair_component(cls, component):
        changed = False
        component_type = getattr(component, "component_type", None)

        if not component_type:
            return False

        for attr, value in list(vars(component).items()):
            path = f"{component_type}.{attr}"

            if path in cls.NUMERIC_RANGES:
                minimum, maximum = cls.NUMERIC_RANGES[path]

                try:
                    numeric = float(value)
                except Exception:
                    continue

                clamped = max(minimum, min(maximum, numeric))

                if clamped != numeric:
                    setattr(component, attr, clamped)
                    changed = True

        return changed

    @classmethod
    def validate_visual_script(cls, component):
        warnings = []
        node_ids = set()

        for node in getattr(component, "nodes", []):
            node_id = node.get("id")

            if not node_id:
                warnings.append(f"VisualScript {component.graph_name} tiene nodo sin id")
                continue

            if node_id in node_ids:
                warnings.append(f"VisualScript {component.graph_name} tiene nodo duplicado: {node_id}")

            node_ids.add(node_id)

        for node in getattr(component, "nodes", []):
            next_id = node.get("next")

            if next_id and next_id not in node_ids:
                warnings.append(f"VisualScript {component.graph_name} apunta a nodo inexistente: {next_id}")

        return warnings
