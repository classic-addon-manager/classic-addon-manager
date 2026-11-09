# Asset/code generators. icons downloads lucide-static into a temporary directory.
# Usage: `just gen icons <names...>`

dir := source_directory()

[private]
default:
    @just --list gen

# Copy lucide icons into crates/cam_ui/assets/icons by name (kebab or PascalCase, DEST=SRC to rename)
icons +NAMES:
    python3 "{{ dir }}/gen_icons.py" {{ NAMES }}
