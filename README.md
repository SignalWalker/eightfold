<div style="font-family: CirrusCumulus, serif; font-size: 4em; display: flex; justify-content: center; margin: auto;" align="center">
  <img src="assets/doc/wordmark.svg" srcset="https://github.com/SignalWalker/eightfold/raw/main/assets/doc/wordmark.svg" alt="Eightfold" role="img">
</div>

<hr align="center"/>

<div align="center" style="margin: auto; display: flex; justify-content: space-evenly; min-width: fit-content; max-width: 72ch;">
  <a href="https://crates.io/crates/eightfold"><img src="https://img.shields.io/crates/v/eightfold" alt="crates.io"/></a>
  <a href="https://github.com/SignalWalker/eightfold/commits/main"><img src="https://img.shields.io/github/commits-since/SignalWalker/eightfold/0.1.0" alt="commits since last release"/></a>
  <a href="https://docs.rs/eightfold"><img src="https://img.shields.io/docsrs/eightfold" alt="docs.rs"/></a>
  <a href="https://opensource.org/licenses/lgpl-license" rel="external license"><img src="https://img.shields.io/crates/l/eightfold" alt="LGPL 3.0 or later"/></a>
</div>

<p align="center">A library for partitioning 3D data. Built with <a href="https://nalgebra.org">nalgebra</a>.</p>

<hr align="center"/>

Not yet fit for actual use; wait until [1.0.0](https://github.com/SignalWalker/eightfold/issues/1).

## Feature Flags

* `spatial` :: [Octree] wrappers with a defined transformation outside of their internal space.
* `render` :: Utilities for rendering an [Octree] with a GPU.
* `tracing` :: Emit trace events using [tracing](https://github.com/tokio-rs/tracing).

## Usage

* [Examples](./samples)

## See Also

* [1.0.0 Checklist](https://github.com/SignalWalker/eightfold/issues/1)
* [Subprojects](./lib)

## References

1. Michael Schwarz and Hans-Peter Seidel. 2010. Fast parallel surface and solid voxelization on GPUs. ACM Trans. Graph. 29, 6, Article 179 (December 2010), 10 pages. <https://doi.org/10.1145/1882261.1866201>
