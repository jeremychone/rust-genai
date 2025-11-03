# Specification Guidelines

This document defines the rules for creating and maintaining specification files.

Important formatting rules

- Use `-` for bullet points. 
- For numbering bullet point style, have empty lines between numbering line. 


## Types of Specification Files

### `spec--index.md`

A single file providing a high-level summary of the entire system.

### `spec-module_name.md`

A specification file for each individual module.  
- `module-path-name` represents the module’s hierarchy path, flattened with `-`.  
- Each file documents the specification for a single module.

Make sure that the `module_name` is the top most common just after `src/`

For example `src/module_01/sub_mod/some_file.rs` the spec module name will be `dev/spec/spec-module_01.md`

(module_name is lowercase)

## Required Structure for Module Specification Files

Each `spec-module-path-name.md` file must include the following sections.

<module_spec_template>

## module-path-name

### Goal

A clear description of the module’s purpose and responsibilities.

### Public Module API

A description of the APIs exposed by the module.  
- Define what is exported and how it can be consumed by other modules.  
- Include function signatures, data structures, or endpoints as needed.

### Module Parts

A breakdown of the module’s internal components.  
- May reference sub-files or sub-modules.  
- Should explain how the parts work together.

### Key Design Considerations

Key design considerations of this module and of its key parts. 



</module_spec_template>
