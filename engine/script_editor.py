import ast
import os


DEFAULT_SCRIPT = '''class NewScript:
    def __init__(self):
        self.script_name = "NewScript"
        self.enabled = True
        self.started = False

    def start(self):
        pass

    def update(self, dt):
        pass
'''


class ScriptDocument:
    def __init__(self, filename, lines=None):
        self.filename = filename
        self.lines = lines or [""]
        self.cursor_line = 0
        self.cursor_col = 0
        self.scroll = 0
        self.dirty = False
        self.syntax_error = None
        self.symbols = []


class ScriptEditor:
    """
    Script Editor avanzado 0.6.
    - tabs de archivos
    - save/save all
    - validación de sintaxis
    - símbolos básicos
    - snippets y autocompletado de API
    - attach to selected
    """

    def __init__(self, game):
        self.game = game
        self.visible = False
        self.documents = []
        self.active_index = 0
        self.show_help = True
        self.show_errors = True
        self.show_symbols = True

        self.suggestions = [
            "entity.x",
            "entity.y",
            "entity.speed",
            "entity.sprite_name",
            "entity.get_component",
            "entity.add_component",
            "entity.game.api.find",
            "entity.game.api.instantiate",
            "entity.game.api.destroy",
            "entity.game.input_map.get_action",
            "entity.game.console.log",
            "dt",
            "self.enabled",
            "self.timer",
        ]

        self.snippets = {
            "move_right": "entity.x += dt * entity.speed",
            "log": 'entity.game.console.log("Hello from script", "SCRIPT")',
            "input_jump": 'if entity.game.input_map.get_action("jump"):\\n    entity.y -= 5 * dt',
            "collision": "def on_collision_enter(self, entity, other):\\n    pass",
            "trigger": "def on_trigger_enter(self, entity, other):\\n    pass",
            "timer": "self.timer = getattr(self, 'timer', 0) + dt",
        }

        self.filename = "scripts/user_script.py"
        self.load_or_create()

    @property
    def document(self):
        if not self.documents:
            self.open_file(self.filename)

        return self.documents[self.active_index]

    @property
    def lines(self):
        return self.document.lines

    @lines.setter
    def lines(self, value):
        self.document.lines = value

    @property
    def cursor_line(self):
        return self.document.cursor_line

    @cursor_line.setter
    def cursor_line(self, value):
        self.document.cursor_line = value

    @property
    def cursor_col(self):
        return self.document.cursor_col

    @cursor_col.setter
    def cursor_col(self, value):
        self.document.cursor_col = value

    @property
    def scroll(self):
        return self.document.scroll

    @scroll.setter
    def scroll(self, value):
        self.document.scroll = value

    def load_or_create(self):
        os.makedirs("scripts", exist_ok=True)

        if not os.path.exists(self.filename):
            with open(self.filename, "w", encoding="utf-8") as file:
                file.write(DEFAULT_SCRIPT)

        self.open_file(self.filename)

    def toggle(self):
        self.visible = not self.visible

    def open_file(self, filename):
        filename = os.path.normpath(filename)

        for index, document in enumerate(self.documents):
            if os.path.normpath(document.filename) == filename:
                self.active_index = index
                self.filename = document.filename
                self.visible = True
                return

        if not os.path.exists(filename):
            os.makedirs(os.path.dirname(filename) or ".", exist_ok=True)

            with open(filename, "w", encoding="utf-8") as file:
                file.write(DEFAULT_SCRIPT)

        with open(filename, "r", encoding="utf-8") as file:
            lines = file.read().splitlines() or [""]

        document = ScriptDocument(filename, lines)
        self.documents.append(document)
        self.active_index = len(self.documents) - 1
        self.filename = filename
        self.validate(document)
        self.visible = True

    def switch_tab(self, delta):
        if not self.documents:
            return

        self.active_index = (self.active_index + delta) % len(self.documents)
        self.filename = self.document.filename

    def close_tab(self):
        if not self.documents:
            self.visible = False
            return

        self.documents.pop(self.active_index)

        if not self.documents:
            self.visible = False
            self.load_or_create()
            self.visible = False
            return

        self.active_index = min(self.active_index, len(self.documents) - 1)
        self.filename = self.document.filename

    def save(self):
        document = self.document

        try:
            os.makedirs(os.path.dirname(document.filename) or ".", exist_ok=True)

            with open(document.filename, "w", encoding="utf-8") as file:
                file.write("\n".join(document.lines) + "\n")
        except Exception as error:
            self.game.console.log(f"No se pudo guardar script: {error}", "ERROR")
            return False

        document.dirty = False
        self.validate(document)
        self.refresh_project_scripts()
        self.game.console.log(f"Script guardado: {document.filename}", "SCRIPT")
        return True

    def save_all(self):
        for index in range(len(self.documents)):
            self.active_index = index
            self.save()

    def refresh_project_scripts(self):
        try:
            self.game.script_manager.scan_scripts(project_path=self.game.project_path)
            self.game.asset_database.scan()
            self.game.file_browser.refresh()

            if hasattr(self.game, "load_project_systems"):
                self.game.load_project_systems()
        except Exception as error:
            self.game.console.log(f"No se pudieron recargar scripts: {error}", "ERROR")

    def validate(self, document=None):
        document = document or self.document
        code = "\n".join(document.lines)
        document.syntax_error = None
        document.symbols = []

        try:
            tree = ast.parse(code, filename=document.filename)
        except SyntaxError as error:
            document.syntax_error = {
                "line": error.lineno or 1,
                "offset": error.offset or 1,
                "message": error.msg,
            }
            return False

        for node in ast.walk(tree):
            if isinstance(node, (ast.ClassDef, ast.FunctionDef)):
                document.symbols.append(
                    {
                        "name": node.name,
                        "line": node.lineno,
                        "type": "class" if isinstance(node, ast.ClassDef) else "def",
                    }
                )

        return True

    def attach_to_selected(self):
        self.save()

        if not self.game.selected_units:
            self.game.console.log("Selecciona una entidad primero", "WARNING")
            return False

        script_name = os.path.splitext(os.path.basename(self.document.filename))[0]
        script = self.game.script_manager.create(script_name)

        if not script:
            self.game.console.log(f"No se pudo crear script: {script_name}", "ERROR")
            return False

        self.game.selected_units[0].add_script(script)
        self.game.history.take_snapshot("Attach Script")
        self.game.mark_scene_dirty("Attach Script")
        self.game.console.log(f"Script attached: {script_name}", "SCRIPT")
        return True

    def insert_char(self, char):
        line = self.lines[self.cursor_line]
        self.lines[self.cursor_line] = line[:self.cursor_col] + char + line[self.cursor_col:]
        self.cursor_col += len(char)
        self.document.dirty = True
        self.validate()

    def insert_text(self, text):
        text = text.replace("\\n", "\n")

        for char in text:
            if char == "\n":
                self.new_line()
            else:
                self.insert_char(char)

    def insert_snippet(self, name):
        snippet = self.snippets.get(name)

        if snippet:
            self.insert_text(snippet)
            self.game.console.log(f"Snippet insertado: {name}", "SCRIPT")

    def autocomplete(self):
        current_word = self.get_current_word()

        if not current_word:
            return

        for suggestion in self.suggestions:
            if suggestion.startswith(current_word):
                self.insert_text(suggestion[len(current_word):])
                self.game.console.log(f"Autocomplete: {suggestion}", "SCRIPT")
                return

    def get_current_word(self):
        line = self.lines[self.cursor_line][:self.cursor_col]
        separators = [" ", "\t", "(", ")", ":", ",", "+", "-", "*", "/", "="]
        start = len(line)

        for index in range(len(line) - 1, -1, -1):
            if line[index] in separators:
                break

            start = index

        return line[start:]

    def new_line(self):
        line = self.lines[self.cursor_line]
        before = line[:self.cursor_col]
        after = line[self.cursor_col:]
        indent = len(before) - len(before.lstrip(" "))

        if before.rstrip().endswith(":"):
            indent += 4

        self.lines[self.cursor_line] = before
        self.lines.insert(self.cursor_line + 1, " " * indent + after)
        self.cursor_line += 1
        self.cursor_col = indent
        self.document.dirty = True
        self.ensure_cursor_visible()
        self.validate()

    def backspace(self):
        if self.cursor_col > 0:
            line = self.lines[self.cursor_line]
            self.lines[self.cursor_line] = line[:self.cursor_col - 1] + line[self.cursor_col:]
            self.cursor_col -= 1
        elif self.cursor_line > 0:
            previous = self.lines[self.cursor_line - 1]
            current = self.lines[self.cursor_line]
            self.cursor_col = len(previous)
            self.lines[self.cursor_line - 1] = previous + current
            self.lines.pop(self.cursor_line)
            self.cursor_line -= 1

        self.document.dirty = True
        self.ensure_cursor_visible()
        self.validate()

    def move_left(self):
        if self.cursor_col > 0:
            self.cursor_col -= 1

    def move_right(self):
        if self.cursor_col < len(self.lines[self.cursor_line]):
            self.cursor_col += 1

    def move_up(self):
        if self.cursor_line > 0:
            self.cursor_line -= 1
            self.cursor_col = min(self.cursor_col, len(self.lines[self.cursor_line]))
            self.ensure_cursor_visible()

    def move_down(self):
        if self.cursor_line < len(self.lines) - 1:
            self.cursor_line += 1
            self.cursor_col = min(self.cursor_col, len(self.lines[self.cursor_line]))
            self.ensure_cursor_visible()

    def scroll_up(self):
        self.scroll = max(0, self.scroll - 1)

    def scroll_down(self):
        self.scroll = min(max(0, len(self.lines) - 1), self.scroll + 1)

    def ensure_cursor_visible(self):
        if self.cursor_line < self.scroll:
            self.scroll = self.cursor_line

        if self.cursor_line > self.scroll + 26:
            self.scroll = self.cursor_line - 26

    def toggle_help(self):
        self.show_help = not self.show_help

    def toggle_errors(self):
        self.show_errors = not self.show_errors

    def toggle_symbols(self):
        self.show_symbols = not self.show_symbols

    def create_new_script(self, name="NewScript"):
        path = self.game.file_browser.create_script(name)

        if path:
            self.open_file(path)

        return path

    def reload_scripts(self):
        self.refresh_project_scripts()
        self.game.console.log("Scripts recargados desde Script Editor", "SCRIPT")

    def run_active(self):
        if not self.save():
            return False

        script_name = os.path.splitext(os.path.basename(self.document.filename))[0]
        script = self.game.script_manager.create(script_name)

        if not script:
            self.game.console.log(f"No se pudo ejecutar script: {script_name}", "ERROR")
            return False

        try:
            if hasattr(script, "start"):
                try:
                    script.start()
                except TypeError:
                    if self.game.selected_units:
                        script.start(self.game.selected_units[0])

            self.game.console.log(f"Run OK: {script_name}", "SCRIPT")
            return True
        except Exception as error:
            self.game.console.log(f"Run error {script_name}: {error}", "ERROR")
            return False
