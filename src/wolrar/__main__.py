from __future__ import annotations

import sys

from wolrar.ui import ArchiveApp


def main() -> None:
    app = ArchiveApp()
    if "--smoke-test" in sys.argv:
        if not app.engine.executable.is_file():
            raise RuntimeError(f"Motore 7-Zip non incluso: {app.engine.executable}")
        app.after(150, app.destroy)
    app.mainloop()


if __name__ == "__main__":
    main()
