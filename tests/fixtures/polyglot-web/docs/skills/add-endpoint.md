Endpoints in this repo are declared in the OpenAPI document first and generated
into the client second. Writing the handler before the schema produces a client
that compiles against an endpoint nobody can call.

## Conventions this repo has that others do not

- **Every endpoint carries a `tags:` entry.** The client generator groups by it,
  and an untagged endpoint lands in a module named after the whole service.
- **4xx bodies use the shared `Problem` shape**, never a bare string. There is
  one endpoint that returns a string and it is a known mistake, not a pattern.

## Judgement calls that are yours

If the endpoint changes an existing response shape rather than adding one, stop
and raise it — deployed clients read it, and that is a two-release change no
amount of local green proves is safe.
