# Specification Rules

This document defines the rules for creating and maintaining module/layer specification files (e.g., `spec-adapter.md`, `spec-tool.md`).

## Formatting Rules

- Use `-` for bullet points. 
- For numbering bullet point style, have empty lines between numbering line. 
- Keep specifications concise; avoid restating code that is obvious from type signatures.
- Use inline code backticks for type names, function names, and file paths.

## Specification Structure

Each specification file should follow this structure.

### 1. Overview

A short paragraph describing the goal of the module or sub-system.

- What problem it solves or what responsibility it owns.
- Where it sits in the overall architecture (e.g., "sits between the client layer and the adapter layer").

### 2. Code Design Pattern

Describe the high-level design pattern(s) used by this module.

- Name the pattern (e.g., trait-based dispatch, builder, strategy, newtype wrapper).
- Explain how the key pieces relate to each other at a conceptual level.
- If the module exposes a streaming or async model, state it here.

### 3. Public API

List the key public types and public functions that are exposed to consumers of this module.

For each type or function:

- State its name and a one-line purpose.
- For public types, list their key public properties.
- Note the design intent (e.g., "newtype over `String` for type safety", "builder for `ChatRequest`").
- If a type has important public methods, list them briefly with their role.
- Group related items under sub-headings if the module exposes many types.

Do not duplicate full signatures; focus on intent and usage patterns.

### 4. Internal Implementation

Describe the internal code design that is not part of the public API.

- Key internal types, traits, or helper functions and their roles.
- How data flows through the internal components.
- Any important invariants, constraints, or conventions that contributors must follow.
- File organization if there are multiple internal files (e.g., `support.rs`, `streamer.rs`).

### 5. Usage Example

Provide a concise code snippet showing the primary way to use the module.

