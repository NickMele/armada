# Landing

**What it is:** How a [Job](job.md)'s work reaches its target — one Job's own pull request, or a Job whose members are Jobs landing in order.

---

**Kind:** Concept.

**Not yet built.** Nothing on this page is in Fleet today beyond the two rows marked as built in the table below. It is written ahead of the code, the way the registries are — see `../practices/half-built.md`.

## A Job's members are Jobs

A change needing three pull requests to three places, landing in a set order, is **one Job whose plan's members are Jobs of their own**. The parent holds the plan and takes one approval. Each member is an ordinary Job with its own worktree, branch, gates and pull request, and the parent is complete when every member has landed.

**A member is not a child of a second kind.** It is a Job that carries a link, which is what `dependencies` already is — see [Job](job.md), Dependency model. What the parent adds is the plan the members come out of, the single approval in front of them, and a completion that waits on all of them.

| What it needs | Where it stands |
| --- | --- |
| Several Jobs, a worktree each | Built. Each member is an ordinary Job |
| A pull request each | Built. Each member delivers on its own step |
| An order between them | Built. A dependent stays `blocked_by_dependency` until the one before it reaches `completed_success` |
| The next one dispatched when the last lands | **Not built.** A dependent becomes dispatchable and is not auto-dispatched |
| The parent complete once every member has landed | **Not built.** `dispatched_by` is provenance and creates no dependency, and nothing completes a parent when its members finish |
| Members in separate repositories | **Not built.** A multi-repo Job is not designed |

## How one member waits on the one before it

An order between members is not one thing. Three repositories give three answers, and only the middle one exists.

| Link | What the later member does | Where it stands |
| --- | --- | --- |
| `stacked` | Branches off the earlier member's branch and opens at once, with no wait. Rebases when that branch lands | **Not built.** Nothing stacks branches |
| `merged` | Waits until the earlier member reaches `completed_success` | Built, as `depends_on`. The later member parks at `blocked_by_dependency` |
| `published` | Waits on an artifact the earlier member's merge produces — a pre-release snapshot, a package tag — rather than on the merge itself | **Not built.** Nothing runs after a member's pull request merges |

**The default is the [Manifest](manifest.md)'s, and a plan overrides it.** Which repositories allow stacking, and which need a snapshot published between pull requests, is a fact about the repository that outlives any one change — which is what a Manifest is for. A plan that says otherwise is overriding the repository, not supplying what it failed to say.

**Landed is the merge** (the owner, 22 Sep 2026). A member counts as landed when its pull request merged — `JobDelivery::landed` in `crates/ipc/src/detail.rs` — and not when it reaches `completed_success`, which a Job whose gate hands off to a person reaches while its pull request is still open. Bridge draws the two apart: the Overview region counts pull requests in, never Jobs that finished.

**`published` needs nothing at the parent's level.** A member is a whole Job with its own workflow, and a step that publishes belongs to that workflow. The parent waits on the member; the member is done when its Job is; the Job is done when it has published. Nothing about the link has to know what publishing means.

## Landing together is a setting, not a shape

**Some changes have no valid intermediate state** — a parser and its generator, coupled sibling packages, tightly coupled services. Confirmed as a real pattern from a live work monorepo, not a hypothetical.

**What serves it is a landing setting, not a Job of its own kind.** "These land together or not at all" sits on the landing rule beside the target, the pull request count, the pull request mode and what completes the Job. It is the one setting that can *refuse* to land rather than choose where.

`atomic` is on the Job record today (`crates/core-model/domain/job-fields.toml`) and is read as a shape discriminator. It stops being one. Whether the land-together setting replaces that field or sits beside it is not settled — see Open questions.

## What the landing rule carries

| Setting | What it chooses | Where it stands |
| --- | --- | --- |
| Target | `main`, or a named branch | **Not built.** `main` by construction, and the merge line takes turns onto it |
| Pull requests | One for the Job, or one per group | **Not built.** One per Job |
| Link between members | `stacked`, `merged` or `published`, per edge, defaulted by the Manifest | Only `merged` exists. Bridge draws all three (`packages/components/src/compositions/JobMembers/JobMembers.tsx`), and a link derived from today's wire always reads `merged` |
| Pull request mode | Ready, or draft | **Not built** |
| Advance at review | Automatic, or a person | Built, as `manifest_rule:auto_merge` on an advance gate |
| Complete when | The pull request lands, or every member has | Only the first exists |
| Lands together or not at all | Members that must land in one commit, against members free to land apart | **Not built** |

**Six of the seven are missing, and that is the size of the idea.** The plan, the members and the order are the cheap half.

**Both ends of the range stay.** Where the work comes from and where it lands are separate settings and are not folded into one: they differ whenever a Job starts from an unmerged branch or lands in a long-lived one.

## One Job, one pull request, stays the default

A Job that names one place to write delivers one pull request onto its target, and nothing above applies to it. That is every Job Fleet runs today.

## Open questions

- **[landing-what-dispatches-the-next-member]** What dispatches the next member when its dependency clears? A dependent becomes dispatchable and is not auto-dispatched, so today a person presses it.
- **[landing-where-land-together-lives]** Where does "land together or not at all" live — a setting on a group, a field replacing `atomic`, or something else? The pattern is confirmed real; only its home is open.
- **[landing-a-target-branch-that-moves]** What happens to a Job whose target branch moves, or is deleted, while it runs? And what does the merge line do with a target that is not `main`, which it takes turns onto by construction?
- **[landing-members-across-repositories]** Does Armada need members in separate repositories, and what is that Job's workflow graph? Carried forward from the root-scoping decision, which left it open, and no scenario currently pressures it.
