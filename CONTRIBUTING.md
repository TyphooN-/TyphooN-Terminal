# Contributing to TyphooN-Terminal

Contributions are welcome. Before submitting a pull request, please read
this document.

## Contributor License Agreement (CLA)

TyphooN-Terminal uses a dual-license model (Business Source License 1.1 +
Commercial License). To maintain the ability to offer both licenses, all
contributors must agree to the [Contributor License Agreement](CLA.md)
before their contributions can be merged.

When you open your first pull request, you will be asked to confirm your
agreement by commenting:

    I have read the CLA and I agree to its terms.

## What the CLA Means

By signing the CLA, you grant the project maintainer a license to use your
contribution under both the BSL 1.1 and the commercial license. You retain
full copyright over your work — you are licensing it, not assigning it.

Your contribution will be:
- Available to the public under BSL 1.1 (and eventually Apache 2.0 after
  the Change Date)
- Potentially included in commercially licensed versions of TyphooN-Terminal

## How to Contribute

1. Fork the repository
2. Create a feature branch from `master`
3. Make your changes
4. Ensure `cargo build` and `cargo test` pass
5. Submit a pull request with a clear description of your changes

## Code Style

- Follow existing patterns in the codebase
- Use `cargo fmt` before committing
- Use `cargo clippy` to catch common issues
- Keep commits focused and atomic

## Scope

TyphooN-Terminal is a Rust trading terminal. Contributions should be
relevant to trading, charting, risk management, broker integrations, or
the terminal infrastructure itself.
