Utilities for serializing [DataSets](crate::DataSet) to [glTF](https://github.com/KhronosGroup/glTF).

# Notes

## glTF Structure

A single set of glTF data is known as an *asset*. Assets contain various properties describing
graphics data.

This is a tree describing the structure of glTF data. Required fields are marked with `(!)`.

* `asset`(!): metadata
  * `version`(!): target glTF format version
  * `minVersion`: minimum required glTF format version
  * `generator`: informal field describing the program used to generate the glTF asset
  * `*`: additional metadata may be stored as desired
* `scene`: index of default scene within the `scenes` list
* `scenes`: list of scenes
  - scene: a set of visual objects to render
    * `name`: scene name
    * `nodes`: list of indices of root-level nodes in this scene. each node *must* be a root. nodes may be reused in multiple scenes.
* `nodes`: list of hierarchical objects within a scene. each node is part of an acyclic tree, and these trees may be disjoint.
  - node: an object within a scene
    * `name`: node name
    * `children`: list of indices to child nodes
    * `translation`: translation from the parent node/scene. (x, y, z). right-handed coordinates. conflicts with `matrix`.
    * `rotation`: rotation from the parent node/scene. unit quaternion, (x, y, z, w). conflicts with `matrix`
    * `scale`: scale from the parent node/scene. (x, y, z). right-handed coordinates.
    * `matrix`: 4x4 transformation matrix from the parent node/scene. Must be decomposable to Translation/Rotation/Scale properties. Conflicts with `translation`, `rotation`, and `scale`. Containing node *must not* be the target of an animation.
    * `mesh`(! if `skin`): index of referenced mesh
    * `skin`: index of referenced mesh skin
* `buffers`: list of arbitrary binary blobs
  - GLB buffer: a special buffer type referring to the binary blob stored in a GLB file. If present, *must* be the first element of the `buffers` array.
    * `byteLength`(!): length of the binary data, in bytes
  - buffer: a reference to a binary blob. *must* use little-endian byte order
    * `byteLength`(!): length of binary data, in bytes
    * `uri`(!): URI to the buffer data. Data may be stored inline using the URI `data:` followed by the base64 encoding of the data with the `mediatype` URI field set to `application/gltf-buffer` or `application/octet-stream`.
* `bufferViews`: list of buffer slices
  - bufferView: a buffer slice
    * `buffer`(!): index of referenced buffer
    * `byteLength`(!): slice length
    * `byteOffset`(!): offset from the beginning of the buffer
    * `byteStride`: when referencing vertex data, this defines the stride in bytes between each vertex. *must not* be defined for non-vertex data.
    * `target`: hint of the intended GPU buffer type for this buffer view. May be either `34962` (for `ARRAY_BUFFER`) or `34963` (for `ELEMENT_ARRAY_BUFFER`).
* `accessors`: list of type definitions for bufferViews
  - accessor: a description of how to retrieve data from a bufferView (i.e. it's a type definition)
    * `bufferView`(!): index of referenced bufferView
    * `byteOffset`(!): offset from the beginning of the bufferView.
    * `count`(!): length of the buffer slice, in elements (the type of which is defined by `type` and `componentType`)
    * `componentType`(!): type of components of the accessed data
      - `5120`: i8
      - `5121`: u8
      - `5122`: i16
      - `5123`: u16
      - `5124`: unsupported by spec (no i32)
      - `5125`: u32
      - `5126`: f32
    * `type`(!): type of accessed data
      - `SCALAR`: 1 component
      - `VEC2`: 2 components
      - `VEC3`: 3 components
      - `VEC4`: 4 components
      - `MAT2`: 4 components
      - `MAT3`: 9 components
      - `MAT4`: 16 components
    * `min`(! if animation input / vertex position accessor): array of per-component minimum values
    * `max`(! if animation input / vertex position accessor): array of per-component maximum values
  - sparse accessor: a special kind of accessor describing small changes to a buffer
    * `bufferView`: index of referenced bufferView. if unspecified, accesses a dummy buffer of all-zeros
    * `byteOffset`(!): offset from the begninning of the bufferView
    * `count`(!): length of the buffer slice, in elements
    * `type`(!): type of the accessed data
    * `componentType`(!): type of components of the accessed data
    * `sparse`(!): description of elements differing from initialized values
      * `count`(!): number of displaced elements. *must* be <= the count of the parent accessor
      * `indices`(!): ???
      * `values`(!): ???
    * `min`(! if animation input / vertex position accessor): array of per-component minimum values (after substitution)
    * `max`(! if animation input / vertex position accessor): array of per-component maximum values (after substitution)
* `meshes`: list of mesh data
  - mesh: collected vertex data
    * `primitives`(!): list of parts of the mesh
      - primitive
        * `attributes`(!): description of vertex attributes & accessors to them. each attribute is an index to an accessor. each accessor must have the same `count`.
          * `POSITION`: [f32; 3]; right-handed; accessor must have defined `min` and `max`
          * `NORMAL`: [f32; 3]
          * `TANGENT`: [f32; 4]; W is -1 or 1, indicating handedness of the tangent basis
          * `TEXCOORD_n`: [(f32 | u8 | u16 ); 2]
          * `COLOR_n`: [(f32 | u8 | u16); (3 | 4)]
          * `JOINTS_n`: [(u8 | u16); 4]
          * `WEIGHTS_n`: [(f32 | u8 | u16); 4]
        * `indices`: accessor index for vertex indices for this primitive. if unspecified, vertices are read in order from each attribute accessor
        * `material`: material index
        * `mode`: topology mode.
          - `0`: `POINTS`; `indices.len` != 0; each vertex is a single point primitive
          - `1`: `LINES`; `indices.len` % 2 == 0 && `indices.len` != 0
          - `2`: `LINE_LOOP`; same as strips, but a final line segment is added from the last vertex to the first; `pₙ = {vₙ, v₁}`
          - `3`: `LINE_STRIP`; `indices.len` >= 2; each line primitive is defined by each vertex -> the following vertex, `pᵢ = {vᵢ, vᵢ₊₁}`
          - `4`: `TRIANGLES`; `indices.len` % 3 == 0 && `indices.len` != 0
          - `5`: `TRIANGLE_STRIP`; `indices.len` >= 3
          - `6`: `TRIANGLE_FAN`; `indices.len` >= 3
        * `targets`: attribute map to accessors containing deltas
          - `POSITION`
          - `NORMAL`
          - `TANGENT`: [f32; 3]
          - `TEXCOORD_n`: [(u8 | u16 | i8 | i16 | f32); 2]
          - `COLOR_n`: [(u8 | u16 | i8 | i16 | f32); (3 | 4)]
    * `weights`: list of weights (f32) to apply to morph targets (`primitives[_].targets`). `weights.len` *must* match the number of morph targets



# See Also

* [glTF Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
