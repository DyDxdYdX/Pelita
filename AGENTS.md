## Documentation-First Development Policy

Pelita follows a lightweight documentation-first development workflow.

Before implementing a new module or making a significant architectural change:

1. Review the relevant existing documentation.
2. Identify missing, outdated, or conflicting requirements.
3. Update the relevant documentation with the intended behavior, architecture, and acceptance criteria.
4. Implement the functionality according to the documented design.
5. Run relevant tests and validate the acceptance criteria.
6. Update the documentation to reflect the actual implementation, including deviations and known limitations.

For small bug fixes, refactoring, or minor UI changes, update documentation only when externally observable behavior, documented interfaces, or architectural decisions change.

Do not rewrite unrelated documentation.

Do not introduce undocumented architectural changes.

When requirements are unclear, record assumptions and request clarification for decisions that materially affect the design.

Treat documentation as a living source of project knowledge, not an immutable specification.
