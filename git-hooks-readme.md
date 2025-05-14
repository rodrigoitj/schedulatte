# Git Hooks

This project uses Git hooks to enforce version control best practices.

## Pre-commit Hook

A pre-commit hook has been set up to prevent commits without updating the version in `Cargo.toml`. This ensures each release has a proper version number.

### How it works

When you attempt to commit changes:

1. The hook checks if `Cargo.toml` is being modified
2. If so, it verifies that the version line has been changed
3. If the version hasn't been updated, the commit will be rejected

### Manual Testing

To test the hook manually:

```bash
# Make some changes to the codebase
git add .

# Try to commit without changing version (should fail)
git commit -m "Test commit"

# Update the version in Cargo.toml
# Then try again (should succeed)
git add Cargo.toml
git commit -m "Updated version"
```

### Skipping the Hook

In rare cases when you need to bypass the hook:

```bash
git commit --no-verify -m "Your commit message"
```

**Note:** Please use this sparingly and only when absolutely necessary.

## Installation

The hooks should be installed automatically when you clone the repository. If you're adding hooks to an existing project, you may need to make them executable:

### Windows

```powershell
# No additional steps needed if using the provided hooks
```

### macOS/Linux

```bash
chmod +x .git/hooks/pre-commit
```
