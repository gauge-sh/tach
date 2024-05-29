from __future__ import annotations

from tach.check import BoundaryError, check
from tach.cli import parse_arguments, print_no_config_yml
from tach.parsing import parse_project_config


def run_tach_check(cwd, argv, source):
    args, _ = parse_arguments(argv[1:])
    root = args.root
    exclude_paths = args.exclude.split(",") if getattr(args, "exclude", None) else None
    try:
        project_config = parse_project_config(root=root)
        if project_config is None:
            print_no_config_yml()

        if exclude_paths is not None and project_config.exclude is not None:
            exclude_paths.extend(project_config.exclude)
        else:
            exclude_paths = project_config.exclude

        boundary_errors: list[BoundaryError] = check(
            root,
            project_config,
            exclude_paths=exclude_paths,
        )
        for boundary_error in boundary_errors:
            # Hack for now - update error message displayed to user.
            error_info = boundary_error.error_info
            if (
                not error_info.exception_message
                and boundary_error.error_info.is_tag_error
            ):
                error_info.exception_message = (
                    f"Cannot import '{boundary_error.import_mod_path}'. "
                    f"Tags {error_info.source_tags} cannot depend on {error_info.invalid_tags}."
                )
    except Exception as e:
        print(str(e))
    return boundary_errors
