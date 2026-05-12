class AudioSystem:
    def __init__(self, game):
        self.game = game
        self.enabled = True
        self.run_in_editor = False
        self.run_in_play = True
        self.stats = {
            "sources": 0,
            "started": 0,
            "missing": 0,
        }

    def update(self, dt):
        mixer = getattr(self.game, "audio_mixer", None)

        if not mixer:
            return

        started = 0
        missing = 0
        sources = 0

        for entity in getattr(self.game.world, "entities", []):
            if not getattr(entity, "enabled", True):
                continue

            source = entity.get_component("AudioSource") if hasattr(entity, "get_component") else None

            if not source or not getattr(source, "enabled", True):
                continue

            sources += 1
            source._entity = entity

            if not getattr(source, "play_on_start", False) or getattr(source, "_started", False):
                continue

            sound = self.game.resources.get_sound(getattr(source, "audio_name", None))

            if not sound:
                missing += 1
                source._started = True
                continue

            mixer.play(
                sound,
                source=source,
                bus_name=getattr(source, "bus", "SFX"),
                volume=getattr(source, "volume", 1.0),
                loop=getattr(source, "loop", False),
            )
            source._started = True
            started += 1

        mixer.update(dt)
        self.stats = {
            "sources": sources,
            "started": started,
            "missing": missing,
        }
