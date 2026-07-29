# Contributing Guide

Thank you for contributing to DCops! This guide will help you get started.

## Getting Started

1. **Fork the repository** on GitHub
2. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-feature
   ```
3. **Make your changes**
4. **Write tests** (TDD is required)
5. **Submit a pull request**

## Development Process

### Test-Driven Development (TDD)

DCops follows strict TDD principles:

1. **Write Tests First** - Write failing tests before implementation
2. **Implement** - Write minimal code to make tests pass
3. **Refactor** - Improve code while keeping tests green
4. **Repeat** - Continue the cycle

### Test Coverage Requirements

- **Minimum:** 65% test coverage
- **Target:** 80% test coverage
- **All code must be tested** - No exceptions

### Code Quality

1. **Run Linters:**
   ```bash
   cargo fmt
   cargo clippy
   ```

2. **Run Tests:**
   ```bash
   cargo test
   cargo nextest run
   ```

3. **Check Coverage:**
   ```bash
   cargo llvm-cov --html
   ```

## Pull Request Process

### Before Submitting

- [ ] All tests pass
- [ ] Test coverage meets minimum (65%)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] Examples updated (if adding new CRDs)

### Pull Request Description

Include:
- **What** - What changes were made
- **Why** - Why the changes were needed
- **How** - How the changes work
- **Testing** - How to test the changes
- **Related Issues** - Link to related issues

### Code Review

- Address all review comments
- Keep PRs focused (one feature per PR)
- Keep PRs small when possible
- Respond to feedback promptly

## Code Style

DCops follows Rust best practices and project-specific guidelines:

- **Formatting:** Use `cargo fmt`
- **Linting:** Use `cargo clippy`
- **Guidelines:** See `rust-guidelines.txt`

📖 **See [Code Style](./code-style.md) for detailed guidelines.**

## Adding New Features

### Adding a New CRD

1. **Define CRD** in `crates/crds/src/`
2. **Generate CRD YAML:**
   ```bash
   python3 scripts/generate_crds.py
   ```
3. **Add Reconciler** in `controllers/netbox/src/reconciler/`
4. **Add Watcher** in `controllers/netbox/src/main.rs`
5. **Write Tests** (TDD)
6. **Add Example CR** in `config/examples/`
7. **Update Documentation** in `ui/src/data/content/`

### Adding a New Controller

1. **Create Controller** in `controllers/<name>/`
2. **Define CRDs** if needed
3. **Implement Reconciler**
4. **Add Deployment** in `config/<name>-controller/`
5. **Update Tiltfile**
6. **Write Tests**
7. **Update Documentation**

## Testing

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_netbox_site_reconciliation

# With nextest (faster)
cargo nextest run

# With coverage
cargo llvm-cov --html
```

### Writing Tests

- **Unit Tests** - Test individual functions
- **Integration Tests** - Test reconciler behavior
- **Mock NetBox API** - Use trait-based mocking

See existing tests in `controllers/netbox/src/reconciler/` for examples.

## Documentation

### User Documentation

User-facing documentation is in `ui/src/data/content/user/`:
- Update guides when adding features
- Add examples to guide pages
- Update CRD reference when adding CRDs

### Contributor Documentation

Contributor documentation is in `ui/src/data/content/contributor/`:
- Update architecture docs when changing design
- Update testing guide when adding test patterns
- Keep contributing guide up to date

### Code Documentation

- Add doc comments to public APIs
- Document complex logic
- Include examples in doc comments

## Git Workflow

DCops uses Conventional Commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Maintenance tasks

**Example:**
```
feat: add NetBoxDevice CRD with full reconciliation support
```

## Getting Help

- **Issues** - Open an issue for bugs or feature requests
- **Discussions** - Use GitHub Discussions for questions
- **Code Review** - Ask questions in PR comments

## Resources

- [Code Style](./code-style.md) - Detailed coding guidelines
- [Development Setup](../development/setup.md) - Development environment
- [Architecture](../development/architecture.md) - System architecture
- [Testing](../development/testing.md) - Testing practices
- [CONTRIBUTING.md](../../../../CONTRIBUTING.md) - Complete contributing guide

---

Thank you for contributing to DCops! 🚀
