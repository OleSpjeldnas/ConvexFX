# Introduction

ConvexFX is a modular foreign-exchange automated market maker implemented in Rust. The project is composed of multiple crates covering pricing, clearing, risk management, fee computation, an HTTP API, and supporting utilities. This book collects developer-facing documentation that explains how to work with the system.

The API reference in the next chapter focuses on the Axum-based HTTP service that exposes ConvexFX functionality to external clients. Each endpoint documents the expected request and the current responses returned by the in-memory reference implementation.
