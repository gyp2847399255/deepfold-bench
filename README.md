# Benchmarking DeepFold

Welcome to the repository dedicated to run all tests that performed in Deepfold.

## Overview

This repository facilitates benchmarking tests for Deepfold.

- **Implementation Details**: PolyFRIM is implemented using $\mathbb{F}_{p^2}$, with $p = 2^{61} - 1$ as the base field and Blake3 as the hash function. The chosen code rate is $2^{-3}$. To modify the code rate, adjust the `CODE_RATE` constant.

- **Provided Implementations**:
  - **PolyFRIM**: The multi-linear FRI-based polynomial commitment scheme proposed in paper. Find this mainly in the `pcs/` directory.
  - **Other FRI-based Multi-linears**: Include Gemini (or HyperPlonk) in `gemini-fri/`, and Virgo in `virgo/` for comparative purposes.
  - **VSS**: One to many univariate polynomial commitment from PolyFRIM, located in the `vss/` directory.
  - **AVSS**: One to many binary polynomail commitment from PolyFRIM, located in the `avss/` directory.

- **Utilities**: All the above protocols leverage utilities found in `util/`, which includes implementations for Merkle trees, finite fields, polynomials, and other necessary tools.

## Setup

1. **Install Rust**: Follow the instructions on [Rust Installation](https://www.rust-lang.org/tools/install).
   
2. **Verify Installation**: Post-installation, ensure everything is set up correctly with:
   ```bash
   cargo --version
   rustup --version
   ```

3. **Use the Nightly Toolchain**: 
   ```bash
   rustup default nightly
   ```

## Benchmarking

- **Benchmark All Protocols**: 
  ```bash
  cargo bench
  ```
  
- **Benchmark a Specific Protocol**: Choose from `vss`, `pcs`, `avss`, `gemini-fri`, or `virgo`.
  ```bash
  cargo bench -p <protocol>
  ```
  
> **Note**: The most extensive benchmarking point may require approximately 50 GB of RAM.

## Running Tests & Determining Proof Size

- **Test All Protocols & Output Proof Sizes**: 
  ```bash
  cargo test -- --nocapture
  ```

- **Test & Output Proof Size for a Specific Protocol**: Choose from `vss`, `pcs`, `avss`, `gemini-fri`, or `virgo`.
  ```bash
  cargo test -p <protocol> -- --nocapture
  ```

## Virgo GKR

For the multi-linear polynomial commitment in Virgo, there's an included GKR.

**Benchmarking GKR**:
1. Execute `bench_gkr.py` within the `virgo/` directory.
2. This script calls the executable `virgo/fft_gkr` and produces the GKR prover time, verifier time, and proof size.

> **Note**: The executable originates from [Virgo](https://github.com/sunblaze-ucb/Virgo), and we're directly utilizing it here.

For the final evaluation result of Virgo, it's essential to sum the results from the Rust implementation and the GKR. This summation is a manual process.