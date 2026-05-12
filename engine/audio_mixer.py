import pygame


class AudioBus:
    def __init__(self, name, volume=1.0, parent="Master"):
        self.name = name
        self.volume = float(volume)
        self.parent = parent
        self.muted = False
        self.solo = False
        self.limit = 16
        self.active_channels = []

    def effective_volume(self, buses):
        if self.muted:
            return 0.0

        volume = self.volume
        parent = buses.get(self.parent)

        if parent and parent is not self:
            volume *= parent.effective_volume(buses)

        return max(0.0, min(1.0, volume))

    def serialize(self):
        return {
            "name": self.name,
            "volume": self.volume,
            "parent": self.parent,
            "muted": self.muted,
            "solo": self.solo,
            "limit": self.limit,
        }


class AudioMixer:
    def __init__(self, game=None):
        self.game = game
        self.buses = {
            "Master": AudioBus("Master", 1.0, "Master"),
            "Music": AudioBus("Music", 0.75),
            "SFX": AudioBus("SFX", 1.0),
            "UI": AudioBus("UI", 0.85),
            "Ambience": AudioBus("Ambience", 0.65),
        }
        self.playing_sources = {}
        self.listener_entity_id = None
        self.initialized = False
        self.stats = {
            "playing": 0,
            "buses": len(self.buses),
            "last_played": None,
        }

    def ensure_initialized(self):
        if self.initialized:
            return True

        try:
            if not pygame.mixer.get_init():
                pygame.mixer.init()
            self.initialized = True
            return True
        except Exception as error:
            self.log(f"Audio mixer no disponible: {error}", "WARNING")
            return False

    def add_bus(self, name, volume=1.0, parent="Master"):
        if name in self.buses:
            return self.buses[name]
        self.buses[name] = AudioBus(name, volume, parent)
        self.stats["buses"] = len(self.buses)
        return self.buses[name]

    def set_bus_volume(self, name, volume):
        bus = self.buses.get(name) or self.add_bus(name)
        bus.volume = max(0.0, min(1.0, float(volume)))

    def toggle_mute(self, name):
        bus = self.buses.get(name)
        if not bus:
            return False
        bus.muted = not bus.muted
        return bus.muted

    def play(self, sound, source=None, bus_name="SFX", volume=1.0, loop=False):
        if not sound or not self.ensure_initialized():
            return None

        bus = self.buses.get(bus_name) or self.add_bus(bus_name)
        loops = -1 if loop else 0
        channel = sound.play(loops=loops)

        if not channel:
            return None

        final_volume = self.compute_source_volume(source, bus, volume)
        channel.set_volume(final_volume)

        source_id = id(source) if source is not None else id(channel)
        self.playing_sources[source_id] = {
            "channel": channel,
            "source": source,
            "bus": bus.name,
            "volume": volume,
            "sound": sound,
        }
        bus.active_channels.append(channel)
        self.stats["last_played"] = getattr(source, "audio_name", None) if source else None
        self.prune_channels()
        return channel

    def stop_source(self, source):
        item = self.playing_sources.pop(id(source), None)

        if item:
            item["channel"].stop()

    def update(self, dt):
        self.prune_channels()

        for item in list(self.playing_sources.values()):
            channel = item["channel"]

            if not channel.get_busy():
                continue

            bus = self.buses.get(item["bus"], self.buses["Master"])
            channel.set_volume(self.compute_source_volume(item["source"], bus, item["volume"]))

        self.stats["playing"] = len(self.playing_sources)

    def compute_source_volume(self, source, bus, volume):
        final = max(0.0, min(1.0, float(volume))) * bus.effective_volume(self.buses)

        if source is None or not self.game:
            return final

        spatial = max(0.0, min(1.0, getattr(source, "spatial_blend", 0.0)))

        if spatial <= 0:
            return final

        entity = getattr(source, "_entity", None)
        listener = self.get_listener_entity()

        if not entity or not listener:
            return final

        dx = getattr(entity, "x", 0.0) - getattr(listener, "x", 0.0)
        dy = getattr(entity, "y", 0.0) - getattr(listener, "y", 0.0)
        distance = (dx * dx + dy * dy) ** 0.5
        min_distance = max(0.001, getattr(source, "min_distance", 4.0))
        max_distance = max(min_distance, getattr(source, "max_distance", 18.0))
        attenuation = 1.0 - max(0.0, min(1.0, (distance - min_distance) / (max_distance - min_distance)))
        return final * ((1.0 - spatial) + spatial * attenuation)

    def get_listener_entity(self):
        if self.listener_entity_id and self.game and hasattr(self.game, "get_entity_by_id"):
            return self.game.get_entity_by_id(self.listener_entity_id)

        selected = getattr(self.game, "selected_units", []) if self.game else []

        if selected:
            return selected[0]

        units = getattr(self.game, "units", []) if self.game else []
        return units[0] if units else None

    def prune_channels(self):
        for source_id, item in list(self.playing_sources.items()):
            if not item["channel"].get_busy():
                self.playing_sources.pop(source_id, None)

        for bus in self.buses.values():
            bus.active_channels = [
                channel for channel in bus.active_channels
                if channel.get_busy()
            ][:bus.limit]

    def serialize(self):
        return {
            "buses": {
                name: bus.serialize()
                for name, bus in self.buses.items()
            },
            "listener_entity_id": self.listener_entity_id,
        }

    def deserialize(self, data):
        for name, bus_data in data.get("buses", {}).items():
            bus = self.buses.get(name) or self.add_bus(name)
            bus.volume = bus_data.get("volume", bus.volume)
            bus.parent = bus_data.get("parent", bus.parent)
            bus.muted = bus_data.get("muted", bus.muted)
            bus.solo = bus_data.get("solo", bus.solo)
            bus.limit = bus_data.get("limit", bus.limit)

        self.listener_entity_id = data.get("listener_entity_id")

    def log(self, message, level="AUDIO"):
        if self.game and hasattr(self.game, "console"):
            self.game.console.log(message, level)
