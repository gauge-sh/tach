### Installation
```bash
pip install tach
```
### Setup
Tach allows you to configure where you want to place module boundaries in your project.

You can do this interactively! From the root of your python project, run:
```bash
 tach mod
# Up/Down: Navigate  Enter: Mark/unmark module  Right: Expand  Left: Collapse  Ctrl + Up: Jump to parent
# Ctrl + s: Exit and save  Ctrl + c: Exit without saving  Ctrl + a: Mark/unmark all
```
Mark and unmark each module boundary you want to create with 'Enter' (or 'Ctrl + a' to mark all sibling modules). Common choices would be to mark all of your top-level Python source packages, or just a few packages which you want to isolate.

If your Python code lives below your project root, you should mark your Python [source root](configuration.md#source-root) using the 's' key.

Once you have marked all the modules you want to enforce constraints between, run:
```bash
tach sync
```
This will create the main configuration file for your project, `tach.yml`, with the dependencies that currently exist between each module you've marked.

You can then see what Tach has found by viewing the `tach.yml`'s contents: 
```
cat tach.yml
```

NOTE: Your 'project root' directory (the directory containing your `tach.yml`) will implicitly be treated as a module boundary, and may show up in your dependency constraints as '<root>'.


### Enforcement
Tach comes with a simple cli command to enforce the boundaries that you just set up! From the root of your Python project, run:
```bash
tach check
```
You will see:
```bash
✅ All module dependencies validated!
```

You can validate that Tach is working by either commenting out an item in a `depends_on` key in `tach.yml`, or by adding an import between modules that didn't previously import from each other. 

Give both a try and run `tach check` again. This will generate an error:
```bash
❌ path/file.py[LNO]: Cannot import 'path.other'. Tags ['scope:other'] cannot depend on ['scope:file']. 
```

### Extras

If an error is generated that is an intended dependency, you can sync your actual dependencies with `tach.yml`:
```bash
tach sync
```
After running this command, `tach check` will always pass.


Tach also supports:
- [Manual file configuration](https://gauge-sh.github.io/tach/configuration/)
- [Strict public interfaces for modules](https://gauge-sh.github.io/tach/strict-mode/)
- [Inline exceptions](https://gauge-sh.github.io/tach/tach-ignore/)
- [Pre-commit hooks](https://gauge-sh.github.io/tach/usage/#tach-install)
