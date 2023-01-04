# Eightfold

<p align="center">
  <img src="wordmark.svg" alt="The Eightfold wordmark; a cube divided into color-coded octants followed by the word 'Eightfold'." title="Eightfold" role="img"/>
</p>

<p align="center">
  <a href="https://crates.io/crates/eightfold"><img src="https://img.shields.io/crates/v/eightfold" alt="crates.io"/></a>
  <a href="https://github.com/SignalWalker/eightfold/commits/main"><img src="https://img.shields.io/github/commits-since/SignalWalker/eightfold/0.1.0" alt="commits since last release"/></a>
  <a href="https://docs.rs/eightfold"><img src="https://img.shields.io/docsrs/eightfold" alt="docs.rs"/></a>
  <a href="https://opensource.org/licenses/lgpl-license"><img src="https://img.shields.io/crates/l/eightfold" alt="LGPL 3.0 or later"/></a>
</p>

A library for spatial partitioning of 3D data. Built with [nalgebra](https://nalgebra.org).

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

\[1\] Michael Schwarz and Hans-Peter Seidel. 2010. Fast parallel surface and solid voxelization on GPUs. ACM Trans. Graph. 29, 6, Article 179 (December 2010), 10 pages. <https://doi.org/10.1145/1882261.1866201>
