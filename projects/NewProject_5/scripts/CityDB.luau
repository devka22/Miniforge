local CityDB = {}

CityDB.world = {
    title = "Neon Sombra",
    city = "Santa Aurelia",
    bounds = { min_x = 0, min_y = 0, max_x = 128, max_y = 80 },
    tagline = "Una ciudad partida entre puerto, centro financiero, marina, playa neon y barrios que se defienden solos.",
}

CityDB.districts = {
    {
        id = "bajo_muelle",
        name = "Bajo Muelle",
        mood = "sal, oxido y favores viejos",
        min_x = 0,
        max_x = 28,
        min_y = 38,
        max_y = 64,
        rumor = "en los muelles nadie habla gratis, pero todos escuchan.",
    },
    {
        id = "marina_sur",
        name = "Marina Sur",
        mood = "gasolina marina, yates cerrados y camaras privadas",
        min_x = 0,
        max_x = 92,
        min_y = 65,
        max_y = 80,
        rumor = "los yates llegan sin luces cuando la marea tapa motores.",
    },
    {
        id = "talleres_sur",
        name = "Talleres Sur",
        mood = "metal caliente, carreras cortas",
        min_x = 0,
        max_x = 42,
        min_y = 0,
        max_y = 37,
        rumor = "los motores sin placas salen antes del amanecer.",
    },
    {
        id = "centro_civico",
        name = "Centro Civico",
        mood = "cristal limpio, miedo limpio",
        min_x = 42,
        max_x = 68,
        min_y = 20,
        max_y = 42,
        rumor = "las camaras giran antes de que llegue una patrulla.",
    },
    {
        id = "mercado_viejo",
        name = "Mercado Viejo",
        mood = "neon barato, memoria cara",
        min_x = 28,
        max_x = 68,
        min_y = 42,
        max_y = 64,
        rumor = "Mara dejo copias donde la lluvia no apaga la tinta.",
    },
    {
        id = "mirador_norte",
        name = "Mirador Norte",
        mood = "dinero viejo y sirenas lejanas",
        min_x = 68,
        max_x = 95,
        min_y = 0,
        max_y = 32,
        rumor = "Rojas no patrulla aqui; aqui lo invitan a cenar.",
    },
    {
        id = "ribera",
        name = "La Ribera",
        mood = "verde oscuro, fiestas caras",
        min_x = 68,
        max_x = 85,
        min_y = 32,
        max_y = 64,
        rumor = "cada lancha trae musica, cajas selladas y una mentira.",
    },
    {
        id = "playa_neon",
        name = "Playa Neon",
        mood = "arena fria, agua negra, turistas y neon rosa",
        min_x = 86,
        max_x = 128,
        min_y = 34,
        max_y = 80,
        rumor = "en Pier 21 nadie distingue una fiesta de una entrega.",
    },
    {
        id = "coral_norte",
        name = "Coral Norte",
        mood = "hoteles altos, valet nervioso y ventanas encendidas",
        min_x = 96,
        max_x = 128,
        min_y = 0,
        max_y = 33,
        rumor = "las suites de lujo pagan por no aparecer en los mapas.",
    },
}

CityDB.characters = {
    Mara = {
        role = "periodista de datos",
        district = "Bajo Muelle",
        line = "Si el mapa de la ciudad miente, lo corregimos con pruebas.",
        trust_line = "Nico, si te siguen, no vuelvas recto. Haz que la ciudad mienta por ti.",
    },
    Ivo = {
        role = "mecanico y antiguo corredor",
        district = "Talleres Sur",
        line = "Nada corre limpio en Santa Aurelia. Ni los motores, ni la policia.",
        trust_line = "Un buen escape no empieza cuando aceleras; empieza tres cuadras antes.",
    },
    Valeria = {
        role = "fiscal anticorrupcion",
        district = "Centro Civico",
        line = "No necesito heroes. Necesito testigos que lleguen vivos.",
        trust_line = "Cada prueba que traes cambia una orden de captura por una audiencia.",
    },
    Rojas = {
        role = "capitan de patrulla",
        district = "Comisaria Norte",
        line = "La ciudad respira mejor cuando todos obedecen.",
        trust_line = "Tu nombre ya esta en mi mesa. Falta decidir si sera expediente o advertencia.",
    },
    Luz = {
        role = "duena de Pier 21 e informante de la marina",
        district = "Playa Neon",
        line = "En la playa todo parece fiesta hasta que miras quien paga la musica.",
        trust_line = "Los yates no atracan por placer, Nico. Atracan cuando alguien debe desaparecer.",
    },
}

CityDB.missions = {
    {
        id = "act1_rain_on_the_bridge",
        title = "Acto I: Lluvia sobre el puente",
        contact = "Mara",
        objective = "Habla con Mara en Bajo Muelle y recoge el sobre del viejo mercado.",
        waypoint = "Mara_Contact",
        beat = "Mara tiene archivos que prueban que la red de camaras fue alterada.",
    },
    {
        id = "act1_engine_for_a_ghost",
        title = "Acto I: Motor para un fantasma",
        contact = "Ivo",
        objective = "Busca a Ivo en Talleres Sur y prepara una ruta de escape.",
        waypoint = "Ivo_Contact",
        beat = "Ivo sabe quien mueve los autos sin placas por debajo del viaducto.",
    },
    {
        id = "act2_the_blue_hour",
        title = "Acto II: La hora azul",
        contact = "Valeria",
        objective = "Lleva las pruebas al Centro Civico sin elevar demasiado el nivel de busqueda.",
        waypoint = "Valeria_Contact",
        beat = "Valeria puede abrir el caso, pero Rojas ya vigila cada calle iluminada.",
    },
    {
        id = "act2_clean_signal",
        title = "Acto II: Senal limpia",
        contact = "Mara",
        objective = "Sabotea el nodo policial y sobrevive al cerco.",
        waypoint = "Signal_Node",
        beat = "La ciudad queda a oscuras por segundos; suficientes para ver quien manda.",
    },
    {
        id = "act2_pier_21",
        title = "Acto II: Pier 21 no duerme",
        contact = "Luz",
        objective = "Encuentra a Luz en Pier 21 y rastrea el yate que apago sus luces.",
        waypoint = "Luz_Contact",
        beat = "Luz vio a Rojas subir a un yate sin matricula mientras la playa seguia bailando.",
    },
    {
        id = "act2_marina_blackbox",
        title = "Acto II: Caja negra en la marina",
        contact = "Luz",
        objective = "Recupera la caja negra del Yacht Club sin disparar la alarma.",
        waypoint = "Luz_Contact",
        beat = "La marina guarda una copia de rutas que no pasan por ninguna carretera.",
    },
    {
        id = "act3_rojas_wall",
        title = "Acto III: El muro de Rojas",
        contact = "Ivo",
        objective = "Vuelve con Ivo, consigue un auto frio y atraviesa los bloqueos del norte.",
        waypoint = "Ivo_Contact",
        beat = "Rojas corta avenidas como si cerrara una mano. Ivo aun conoce los dedos rotos.",
    },
    {
        id = "act3_open_file",
        title = "Acto III: Archivo abierto",
        contact = "Valeria",
        objective = "Entrega el archivo final a Valeria antes de que la red borre las copias.",
        waypoint = "Valeria_Contact",
        beat = "Valeria convierte rumores en cargos. Santa Aurelia empieza a mirar de vuelta.",
    },
}

CityDB.dispatch = {
    decay_per_second = 0.035,
    tiers = {
        { level = 1, label = "Sospecha", response = "patrullas curiosas" },
        { level = 2, label = "Busqueda", response = "patrullas coordinadas" },
        { level = 3, label = "Cerco", response = "bloqueos y persecucion" },
        { level = 4, label = "Alerta roja", response = "toda la red policial" },
        { level = 5, label = "Ciudad cerrada", response = "Rojas toma el mando" },
    },
}

CityDB.weather = {
    {
        id = "clear",
        name = "Claro",
        radio = "cielo limpio sobre la bahia.",
        ambient = 1.0,
        bloom = 0.0,
        rain = 0.0,
        fog = 0.0,
        traffic = 1.0,
        headlights = false,
    },
    {
        id = "cloudy",
        name = "Nublado",
        radio = "nubes bajas entran por Coral Norte.",
        ambient = 0.84,
        bloom = 0.04,
        rain = 0.0,
        fog = 0.18,
        traffic = 0.94,
        headlights = false,
    },
    {
        id = "rain",
        name = "Lluvia",
        radio = "la lluvia lava placas, no culpas.",
        ambient = 0.66,
        bloom = 0.14,
        rain = 0.72,
        fog = 0.24,
        traffic = 0.82,
        headlights = true,
    },
    {
        id = "storm",
        name = "Tormenta suave",
        radio = "truenos sobre Pier 21; las sirenas suenan mas cerca.",
        ambient = 0.52,
        bloom = 0.22,
        rain = 1.0,
        fog = 0.38,
        traffic = 0.72,
        headlights = true,
    },
    {
        id = "coastal_fog",
        name = "Niebla costera",
        radio = "la niebla tapa la marina y deja oir motores.",
        ambient = 0.60,
        bloom = 0.18,
        rain = 0.12,
        fog = 0.78,
        traffic = 0.76,
        headlights = true,
    },
}

CityDB.radio = {
    "Radio Puerto: la niebla entra baja por los muelles.",
    "Onda 77: congestion en Avenida Cobalto.",
    "Canal civico: nuevas camaras activas en Centro Civico.",
    "Frecuencia libre: si ves luces azules, no corras en linea recta.",
    "FM Playa: Pier 21 sigue abierto aunque el cielo cierre.",
    "Bahia 101: yates sin matricula cruzan frente a la costa.",
}

CityDB.menu_mottos = {
    "La luz no perdona. La ciudad tampoco.",
    "Cada calle tiene dos mapas: el oficial y el verdadero.",
    "Rojas vigila las avenidas; los barrios vigilan a Rojas.",
    "Corre cuando debas. Escucha cuando puedas.",
}

CityDB.pedestrian_lines = {
    calm = {
        "No mires a las patrullas.",
        "El mercado cierra temprano desde que llego Rojas.",
        "Dicen que Mara tiene una copia del archivo.",
        "Las luces del puente fallan justo antes de cada redada.",
        "En Ocean Drive los turistas creen que todo es parte del show.",
        "Si la marea sube, los yates salen sin despedirse.",
    },
    wanted = {
        "Luces azules en camino. Bajen la voz.",
        "Otra vez cerraron Cobalto. Eso no es trafico, es caceria.",
        "Si Rojas saca bloqueos, alguien toco un nervio.",
        "No corras recto. Las camaras aman las lineas rectas.",
        "Si cierran Pier 21, sal por la arena, no por la avenida.",
    },
    panic = {
        "Al suelo, al suelo!",
        "Cierren las persianas!",
        "No vi nada. No vi a nadie.",
        "La central ya tiene tu cara!",
        "Apaguen la musica, vienen patrullas!",
    },
    story = {
        "Mara no imprime dos veces el mismo secreto.",
        "Ivo guarda motores como otros guardan pecados.",
        "Valeria entra al juzgado sin escolta. Eso da mas miedo.",
        "El nodo de senal canta de noche.",
        "En Playa Neon nadie distingue una fiesta de una entrega.",
        "Luz sabe que yate llego sin luces y que patrulla no lo vio.",
        "Pier 21 late como una radio clandestina.",
    },
}

CityDB.contact_dialogue = {
    Mara = {
        "El archivo no es una foto. Es una ciudad entera editada a mano.",
        "Rojas no borra crimenes; borra trayectos. Sigue los huecos.",
        "Si la radio libre repite mi nombre, ya vamos tarde.",
    },
    Ivo = {
        "Un coche caliente atrae patrullas. Uno frio atrae preguntas. Escoge veneno.",
        "Tengo rutas por talleres, mercados y patios. Ninguna es limpia, todas sirven.",
        "Cuando la sirena suba, no frenes en luz blanca. Te encajonan.",
    },
    Valeria = {
        "Tu reputacion abre puertas que mi cargo no puede tocar.",
        "Si llegan testigos vivos, Rojas deja de ser rumor y pasa a expediente.",
        "No necesito una confesion. Necesito una cadena de custodia que respire.",
    },
    Luz = {
        "Pier 21 parece fiesta porque el miedo baila mejor con bajo.",
        "Vi un yate entrar sin luces. Luego Rojas pidio que nadie revisara la marina.",
        "Si llueve, los muelles hablan mas claro. El agua marca las huellas.",
    },
}

CityDB.witness_lines = {
    "Testigo anonimo: lo vi cruzar cerca de %s.",
    "Central: vecino reporta movimiento en %s.",
    "Radio corta: alguien acaba de vender una ubicacion en %s.",
}

function CityDB.mission_by_id(id)
    for _, mission in ipairs(CityDB.missions) do
        if mission.id == id then
            return mission
        end
    end
    return nil
end

function CityDB.mission_at(index)
    if index < 1 then
        return CityDB.missions[1]
    end
    if index > #CityDB.missions then
        return CityDB.missions[#CityDB.missions]
    end
    return CityDB.missions[index]
end

function CityDB.weather_at(index)
    if index < 1 then
        return CityDB.weather[1]
    end
    if index > #CityDB.weather then
        return CityDB.weather[#CityDB.weather]
    end
    return CityDB.weather[index]
end

function CityDB.weather_by_id(id)
    for _, weather in ipairs(CityDB.weather) do
        if weather.id == id then
            return weather
        end
    end
    return CityDB.weather[1]
end

function CityDB.district_at(x, y)
    for _, district in ipairs(CityDB.districts) do
        if x >= district.min_x and x <= district.max_x and y >= district.min_y and y <= district.max_y then
            return district
        end
    end
    return CityDB.districts[1]
end

function CityDB.dispatch_label(wanted)
    local tier = CityDB.dispatch.tiers[1]
    for _, candidate in ipairs(CityDB.dispatch.tiers) do
        if wanted >= candidate.level then
            tier = candidate
        end
    end
    return tier.label .. " - " .. tier.response
end

function CityDB.radio_line(wanted, mission_index, district_name)
    if wanted >= 3 then
        local index = (math.floor(wanted * 10 + mission_index) % #CityDB.pedestrian_lines.wanted) + 1
        return "Radio libre: " .. CityDB.pedestrian_lines.wanted[index]
    end
    local index = ((mission_index * 2) % #CityDB.radio) + 1
    if district_name then
        return CityDB.radio[index] .. " / " .. district_name .. " murmura."
    end
    return CityDB.radio[index]
end

function CityDB.pedestrian_line(wanted, mission_index, persona, panic, district_name)
    local bucket = CityDB.pedestrian_lines.calm
    if panic then
        bucket = CityDB.pedestrian_lines.panic
    elseif wanted >= 2.0 then
        bucket = CityDB.pedestrian_lines.wanted
    elseif mission_index >= 2 then
        bucket = CityDB.pedestrian_lines.story
    end
    local seed = mission_index + math.floor(wanted * 3) + string.len(persona or "vecino") + string.len(district_name or "")
    local line = bucket[(seed % #bucket) + 1]
    if district_name == "Playa Neon" and wanted < 2.0 and not panic then
        local beach = {
            "La musica de Pier 21 tapa demasiadas conversaciones.",
            "Los yates bonitos siempre salen antes de las redadas.",
            "La arena guarda huellas mejor que el asfalto.",
        }
        line = beach[(seed % #beach) + 1]
    end
    if district_name and wanted < 2.0 and not panic then
        line = line .. " " .. district_name .. " escucha."
    end
    return line
end

function CityDB.contact_line(contact_id, mission_index, reputation)
    local lines = CityDB.contact_dialogue[contact_id]
    if not lines then
        local character = CityDB.characters[contact_id]
        return character and character.line or "..."
    end
    local index = ((mission_index or 1) + math.floor((reputation or 0) / 2)) % #lines
    return lines[index + 1]
end

function CityDB.menu_motto(mission_index, wanted)
    local index = ((mission_index or 1) + math.floor((wanted or 0) * 2)) % #CityDB.menu_mottos
    return CityDB.menu_mottos[index + 1]
end

return CityDB
