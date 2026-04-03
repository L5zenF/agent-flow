# Visual Rule Graph Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a React Flow-based global visual rule graph that controls provider selection, model selection, path rewrite, and header mutation for the gateway.

**Architecture:** Extend the Rust config schema with a `rule_graph` DSL, validate and execute that graph at request time, and add a `Rule Graph` editor view in the React admin UI using `reactflow`. The graph becomes the primary rule entrypoint, while legacy routes and header rules remain as fallback.

**Tech Stack:** Rust, Axum, Serde, TOML, React, TypeScript, Vite, React Flow

---
