# Contributing to pifp-stellar

Thank you for your interest in contributing to the **Proof-of-Impact Funding Protocol**!

We aim to create high-quality, impactful contributions. To ensure this, we follow the "Meaningful Issues" guidelines.

## 1. Finding an Issue

Please check our [Issues](https://github.com/SoroLabs/pifp-stellar/issues) tab.
We explicitly tag issues with complexity levels to set expectations.

### Complexity & Points

- **Trivial (100 points)**: Simple UI updates, minor bug fixes, documentation typos.
- **Medium (150 points)**: Standard features, adding tests, refining existing logic.
- **High (200 points)**: Architecture changes, new smart contracts, complex integrations.

## 2. Claiming an Issue & ETA (Required)

When you pick up an issue, **you must comment with an estimated time of completion (ETA).**
If you do not provide an ETA, the maintainers may reassign the issue.

> **Example Comment:** "I'm picking this up. ETA: 3 days."

This helps us coordinate and ensures progress isn't stalled.

## 3. Submitting a Pull Request (PR)

- Fork the repository.
- Create a feature branch: `git checkout -b feature/your-feature-name`.
- Commit your changes with meaningful messages: `feat: implement donation logic`.
- Push to your branch and open a PR.
- **Link the Issue**: In your PR description, write `Closes #123`.

## 4. Code Standards

- **Rust/Soroban**: Run `cargo clippy` and `cargo fmt` before submitting.
- **Testing**: Ensure new features include unit tests.

### Local Quality Gates

To ensure code quality and reduce CI noise, we provide local pre-commit hooks. These hooks run automatically before you commit, checking for:

- Trailing whitespace and end-of-file consistency.
- Correct YAML/TOML syntax.
- Rust formatting (`cargo fmt`) and lints (`cargo clippy`).

#### Installation

Run the following command to set up the gates:

```bash
./scripts/install-hooks.sh
```

_Note: If you have `pre-commit` installed (`pip install pre-commit`), it will be used. Otherwise, a standard git hook will be configured as a fallback._

Let's build a trustless future for funding together! 🚀
