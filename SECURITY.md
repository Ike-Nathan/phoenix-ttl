# Security policy

## Supported versions

Phoenix is pre-1.0. Only the latest release receives fixes.

## Reporting a vulnerability

Please report vulnerabilities **privately** through GitHub's [private vulnerability reporting](https://github.com/Ike-Nathan/phoenix-ttl/security/advisories/new). Do not open a public issue. You will get an acknowledgement within 5 working days, and we will agree on a disclosure timeline with you.

## Scope

Phoenix is a test-only library: it is used as a dev-dependency and never compiled into contracts. The main security-relevant risk is a **false pass**, where Phoenix reports that state stays live when it would in fact be archived. Examples: an entry kind it fails to collect, wrong ownership mapping, or an off-by-one at the archival boundary. Please report these privately if you believe they could lead a project to ship a contract that loses state. Ordinary bugs can go in public issues.

Phoenix is not an audit. A passing Phoenix test covers the TTL policy you wrote tests for, under the test environment's ledger settings, and nothing more.
