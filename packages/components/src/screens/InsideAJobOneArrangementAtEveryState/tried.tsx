/**
 * What three attempts at one failure left behind.
 *
 * **Its own file.** This is a page of evidence and the story it sat in exists
 * to make one claim about one moment — that a step out of attempts is holding
 * rather than broken. A fixture file per subject is the pattern this directory
 * keeps, and it is what holds the stories under the length the gate refuses.
 */
export const WHAT_IT_TRIED = (
            <>
              <div className="armada-screen__sunken">
                <span className="armada-screen__eyebrow">The failure, every time</span>
                <pre className="armada-screen__output">{`FAIL settings::selectors::visible_manifests_memoises
  assert_eq!(a, b) — expected the same reference on repeat calls
  left:  Manifests([..]) @0x7f9c2a
  right: Manifests([..]) @0x7f9c31
  packages/settings/test/selectors.test.ts:112`}</pre>
                <p className="armada-screen__caption" data-note>
                  The same assertion, at the same line, on all three attempts.
                </p>
              </div>
              <div className="armada-screen__sunken">
                <span className="armada-screen__eyebrow">What it tried, and what it said it was doing</span>
                <p className="armada-screen__why">
                  Attempt 1 · +18 −4 selectors.ts · same failure — memoised on the selector itself
                  with a module-level cache.
                </p>
                <p className="armada-screen__why">
                  Attempt 2 · +22 −18 selectors.ts · same failure — replaced the cache with a WeakMap
                  keyed on the state object.
                </p>
                <p className="armada-screen__why">
                  Attempt 3 · +6 −22 selectors.ts · same failure — went back to the module cache and
                  widened the key.
                </p>
                <p className="armada-screen__recourse">
                  Three different fixes, one unchanged failure. It is caching in the wrong place, not
                  caching wrongly.
                </p>
              </div>
            </>
);
