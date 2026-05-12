import os
import sys
import argparse

BASE_DIR = os.path.dirname(os.path.abspath(__file__))

if BASE_DIR not in sys.path:
    sys.path.append(BASE_DIR)

from engine.project_launcher import ProjectLauncher
from engine.project_system import ProjectSystem


def run_project(project_path=None, runtime=False, use_launcher=True):
    if runtime:
        os.environ["MINIFORGE_RUNTIME"] = "1"

    if project_path is None and use_launcher:
        launcher = ProjectLauncher()
        project_path = launcher.run()

    if project_path is None:
        project_path = os.getcwd()

    project_system = ProjectSystem()
    project_system.open_project(project_path)
    project_system.apply_project_as_working_directory()

    if BASE_DIR not in sys.path:
        sys.path.append(BASE_DIR)

    from core.game import Game

    game = Game(runtime_mode=runtime)
    game.run()


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description="MiniForge legacy Python launcher")
    parser.add_argument("--project", help="Ruta del proyecto a abrir")
    parser.add_argument("--runtime", action="store_true", help="Ejecuta sin UI de editor")
    parser.add_argument("--no-launcher", action="store_true", help="Abre el proyecto directo")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    run_project(
        project_path=args.project,
        runtime=args.runtime,
        use_launcher=not args.no_launcher and args.project is None,
    )


if __name__ == "__main__":
    main()

