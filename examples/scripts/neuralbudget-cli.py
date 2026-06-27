#!/usr/bin/env python3
"""NeuralBudget Dashboard & CLI TUI entry point.

Usage:
    neuralbudget-dashboard              # Start web dashboard on http://localhost:8080
    neuralbudget-dashboard --port 3000  # Use custom port
    neuralbudget-dashboard --host 0.0.0.0  # Listen on all interfaces
    
    neuralbudget-tui                    # Start CLI TUI
    neuralbudget-tui --demo             # Run demo mode
"""

import sys
import argparse
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="NeuralBudget Dashboard & CLI TUI"
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Dashboard command
    dashboard_parser = subparsers.add_parser("dashboard", help="Run web dashboard")
    dashboard_parser.add_argument(
        "--host",
        default="127.0.0.1",
        help="Server host (default: 127.0.0.1 for local-only)",
    )
    dashboard_parser.add_argument(
        "--port",
        type=int,
        default=8080,
        help="Server port (default: 8080)",
    )
    dashboard_parser.add_argument(
        "--title",
        default="NeuralBudget Dashboard",
        help="Dashboard title",
    )
    dashboard_parser.add_argument(
        "--reload",
        action="store_true",
        help="Enable auto-reload on file changes (development)",
    )

    # TUI command
    tui_parser = subparsers.add_parser("tui", help="Run CLI TUI")
    tui_parser.add_argument(
        "--demo",
        action="store_true",
        help="Run demo mode (no Textual required)",
    )
    tui_parser.add_argument(
        "--textual",
        action="store_true",
        help="Force Textual app (requires Textual)",
    )

    args = parser.parse_args()

    # If no command specified, show help or default to dashboard
    if args.command is None:
        if "--help" not in sys.argv and "-h" not in sys.argv:
            logger.info("Starting dashboard on http://127.0.0.1:8080")
            args.command = "dashboard"
        else:
            parser.print_help()
            return

    if args.command == "dashboard":
        try:
            from neuralbudget.dashboard import Dashboard

            logger.info(f"Starting dashboard on http://{args.host}:{args.port}")
            logger.info("Press Ctrl+C to stop")
            dashboard = Dashboard(
                host=args.host,
                port=args.port,
                title=args.title,
            )
            dashboard.run(reload=args.reload)
        except KeyboardInterrupt:
            logger.info("Shutting down...")
        except ImportError:
            logger.error("FastAPI is required to run dashboard")
            logger.error("Install with: pip install fastapi uvicorn")
            sys.exit(1)

    elif args.command == "tui":
        try:
            from neuralbudget.cli_tui import CliTui

            logger.info("Starting CLI TUI")
            tui = CliTui()

            if args.textual:
                try:
                    tui.run_textual_app()
                except ImportError:
                    logger.error("Textual is required for full TUI")
                    logger.error("Install with: pip install textual")
                    sys.exit(1)
            elif args.demo:
                tui.run_demo()
            else:
                tui.run()

        except KeyboardInterrupt:
            logger.info("Shutting down...")
        except ImportError:
            logger.error("Rich is required to run CLI TUI")
            logger.error("Install with: pip install rich")
            sys.exit(1)


if __name__ == "__main__":
    main()
