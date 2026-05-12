import os
import sys


def run(project_path=None):
    """
    Runtime entrypoint separado del editor.
    """

    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    if base_dir not in sys.path:
        sys.path.insert(0, base_dir)

    from main import run_project

    run_project(
        project_path=project_path or os.getcwd(),
        runtime=True,
        use_launcher=False,
    )


if __name__ == "__main__":
    run()
