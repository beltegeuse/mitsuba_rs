# mitsuba_rs

Rust crate to parse [Mitsuba](https://www.mitsuba-renderer.org/) scenes files. The main objective of this crate is its practicability as all Mitsuba's object (shapes, bsdfs, emitters, ...) have all their fields filled. If the value is not found inside the scene file, the default value is used. 

Note that this crate does not provide a standard way to load obj and ply. However, this crate provides a way to load `serialized` shapes via the feature `serialized`.  

This parser is tested and integrated inside [rustlight](https://github.com/beltegeuse/rustlight) to check its correctness and usability. Please refer to rustlight to see how to use the crate for the moment.

## Known issues

- I do not plan to support full-spectral data yet.
- Environment maps are not supported.
- Some BSDFs are missing (i.e., bumpmap).
- The code panic for some scene. This is the desired behavior for now. This helps to identify which features are missing to load scenes correctly.