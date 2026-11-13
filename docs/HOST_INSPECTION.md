# Read-only Linux host inspection (phase 9)

Q-035 and ADR-0035 authorize an explicit owner invocation of a closed inventory query
set on a Linux controller and Linux target. Nothing runs from audit polling, the dashboard, a model or MCP. This feature neither
installs software nor changes remote configuration. The owner provisions the restricted
account and approves the host by selecting its local profile for each invocation.

## Supported inventory and limits

- **OS:** read `/etc/os-release` as data; keep only validated `ID`/`VERSION_ID` tokens.
  The file is never sourced or evaluated. Other keys are discarded.
- **Kernel:** fixed `/usr/bin/uname -sr`; require Linux and retain only its release token.
- **Listeners:** fixed `/usr/bin/ss -H -lntu`; retain TCP/UDP ports and coarse LOOPBACK,
  WILDCARD, PRIVATE or OTHER bind scope. Drop IPs, interfaces, peers and queue sizes.
  Process arguments/identities are never requested. UDP means a bound UDP socket.

There is no command/argument template, shell, privileged query, log/environment/file
browser, package inventory, vulnerability lookup, remediation or remote application test.
Inventory is not a vulnerability finding, a firewall check, internet reachability proof
or an ASVS safety verdict. Containers/namespaces visible to the account may limit coverage.
Failed/malformed/unsupported queries are explicit failures, never empty successful lists.

Each query has a 10-second connection timeout, a 20-second total SSH deadline and a
64-KiB output cap. Remote command helpers have a five-second deadline. At most 512 socket
records are accepted before deduplication. Overflows fail rather than silently truncate.
Every started command is persisted before SSH, including attempts that later fail.

## Provision the remote account (owner/administrator)

Use a Linux endpoint reachable through the owner's VPN/private network. OpenTatu accepts
only literal private, loopback, link-local or CGNAT addresses, including private IPv6;
no DNS, public address, jump host or proxy. The OpenTatu controller remains separate from
production applications. No real target is configured or provisioned by this change.

1. Install the **same OpenTatu binary** as `/usr/local/bin/opentatu`, root-owned and 0755.
   Install the distribution's OpenSSH server and iproute2 through normal administration.
   The installed binary's runtime libraries must be compatible with that host.
2. Create a dedicated `opentatu-inspect` account with its own primary group, no supplementary
   groups/sudo, no password authentication and a root-owned non-writable home. Use `/bin/sh`
   as the login shell: OpenSSH uses the login shell to start a forced command. A nologin
   shell also rejects the forced collector. Check that the local account/PAM policy permits
   public-key login without permitting password login. The collector itself refuses UID 0.
3. Generate a dedicated Ed25519 client key on the controller with `ssh-keygen`; keep the
   private file owner-only (0400/0600), regular, at an absolute path. No symlink credential,
   agent, certificate, hardware/token prompt or interactive passphrase is supported in this
   initial noninteractive collector. Never reuse an administrative/default SSH identity.
4. Store only that public client key in a root-managed authorized-keys file, for example
   `/etc/ssh/authorized_keys/opentatu-inspect`. Prefix its line with:

   ```text
   restrict,command="/usr/local/bin/opentatu host collect" ssh-ed25519 <DEDICATED_CLIENT_PUBLIC_KEY>
   ```

5. Apply a restricted account stanza in sshd configuration (validate with `sshd -t` before
   the administrator reloads it; maintain a separate administrative session):

   ```text
   Match User opentatu-inspect
       AuthorizedKeysFile /etc/ssh/authorized_keys/opentatu-inspect
       AuthenticationMethods publickey
       PasswordAuthentication no
       KbdInteractiveAuthentication no
       DisableForwarding yes
       PermitTTY no
       PermitTunnel no
       PermitUserRC no
       ForceCommand /usr/local/bin/opentatu host collect
   ```

   Keep `PermitUserEnvironment no` globally; do not accept client environment variables
   that can affect startup, executable/library loading or OpenTatu configuration. The
   controller sends no environment. Root must own the home, binary, configuration and
   authorized-keys path; the inspection account must not change them. Do not grant it
   another unrestricted key, login path, sudo policy or writable startup script.
6. Independently obtain/verify the server's Ed25519 public host key through the owner's
   administrative console. Do not trust an unauthenticated `ssh-keyscan` result or TOFU.
   The client writes a private temporary known_hosts file from this pinned public key and
   uses strict checking, with global trust/configuration/agents/forwarding disabled.

`ForceCommand` alone does not disable forwarding. `restrict` plus the daemon restrictions
apply to a custom or stolen-key client too; client flags alone are insufficient. The
server/administrator and its root-owned collector remain trusted: SSH authenticates the
endpoint, not an attestation of its OS or honesty. SSH authentication may itself update
server security logs; the collector's permitted operations are read-only.

## Configure and invoke

Copy [hosts.example.toml](../config/hosts.example.toml) to a private owner-controlled
`hosts.local.toml`, outside analyzed repositories. Replace every example value, especially
the public host-key placeholder. Only explicit `--hosts-config` loads it. The schema is
strict; it accepts no passwords, key material, command strings, environment or extra fields.

```bash
cargo build -p opentatu --release
./target/release/opentatu --hosts-config /absolute/path/hosts.local.toml host profile --app abastevo --profile abastevo-vpn
./target/release/opentatu --hosts-config /absolute/path/hosts.local.toml host inspect --app abastevo --profile abastevo-vpn
./target/release/opentatu host history --app abastevo --limit 10
cargo xtask check
```

The app must already be registered. `profile` prints the non-secret policy digest and
complete command set without connecting. Review its endpoint/key/account in your local
configuration before `inspect`. Each `inspect` is a new explicit owner authorization, not
a standing grant. It prints minimized JSON and exits nonzero if any query failed; `history`
needs no SSH configuration or connection. The app page shows the five latest summaries;
port display is capped at 32 with a visible note, while history JSON retains the complete
bounded inventory. No raw stdout/stderr, local credential path or host address is stored
in history, model context or application reports.

An inspection has an app-local number, owner start time, profile/digest, command starts,
command completions and finish time from the controller's injected Clock. It is separate
from exact-commit audit IDs. Failure categories are CONFIGURATION, UNAVAILABLE, TRANSPORT,
TIMEOUT, OUTPUT_LIMIT or INVALID_RESPONSE. Transport intentionally combines connection,
host-key, authentication and remote-command failures without exposing raw diagnostics.
An unfinished row means running or interrupted; it is never silently resumed or retried.
A new explicit invocation gets a fresh number. Completed identities/results cannot be
rewritten or deleted. Correct mistakes through new history, not direct SQL edits (R04).

## Rotation and revocation

Keep both the account and private key scoped to this inventory. To rotate, generate a
fresh dedicated client key, replace the root-managed restricted authorized-key line,
update the local credential path and review the new digest before the next invocation.
If a key is suspected compromised, revoke its authorized-key line immediately on the
server; existing OpenTatu history is retained. Revoke/disable active SSH sessions through
the administrator when needed. There is no connection reuse/multiplexing in OpenTatu.
For a server-key rotation, independently verify the new host key and update the explicit
pin. A changed/unrecognized key fails closed. A local file-permission failure occurs
before opening a connection and is still recorded as a failed command attempt.

## Disposable local acceptance

Normal tests use `testing::FakeHosts`, in-memory SQLite and FixedClock, with no SSH/network.
The opt-in tests below connect only to loopback and require the following disposable
fixture, associated with Abastevo. They never execute its application or access production.

The fixture source is [tests/host-fixture](../crates/scanners/tests/host-fixture/Dockerfile).
Build a context under ignored `data/phase9-validation/image/` with that Dockerfile and
sshd_config, the controller's public `/etc/ssl/certs/ca-certificates.crt` CA bundle,
the current compiled binary as `opentatu`, and a newly generated **synthetic
client public key only** as `authorized_key.pub` (with the `restrict,command=...` prefix
above). Keep the private client key outside the build context, 0600. The image installs
public OpenSSH/iproute2 dependencies; it contains no application code, controller data,
real credentials or provider key. It contains a deliberately invalid `.env` under the
root-owned account home to prove that the forced collector bypasses dotenv.

```bash
docker build -t opentatu-phase9-ssh-fixture:local data/phase9-validation/image
docker run -d --name opentatu-phase9-ssh-fixture --read-only --tmpfs /run:rw,nosuid,nodev,size=16m --tmpfs /tmp:rw,nosuid,nodev,size=16m --memory=128m --cpus=1 --pids-limit=64 -p 127.0.0.1::2222 opentatu-phase9-ssh-fixture:local
docker port opentatu-phase9-ssh-fixture 2222
docker exec opentatu-phase9-ssh-fixture cat /etc/ssh/ssh_host_ed25519_key.pub
```

Create ignored `data/phase9-validation/hosts.local.toml` with profile `local-host`, app
`abastevo`, address `127.0.0.1`, the assigned local port, dedicated user, absolute synthetic
private-key path and the host public key obtained from this controlled local container.
Then run:

```bash
cargo test -p opentatu-worker --test host_inspection r12_actual_abastevo_ssh_inventory_and_wrong_host_key -- --ignored --nocapture
cargo test -p opentatu-scanners r12_actual_ssh_forced_collector_denies_shell_subsystem_and_forwarding -- --ignored --nocapture
```

The first requires successful OS/kernel/listener collection and explicit partial failure
with a wrong server key, while retaining both inspections. The second uses a custom client
to attempt empty/shell/write/extra-argument/injection requests and forwarding, all refused.
For administrative acceptance, inspect `sshd -T` on this fixture, verify no forbidden
marker file exists, revoke its synthetic authorized-key line and run another inspection:
all queries must fail as TRANSPORT. With a read-only fixture, use an empty root-owned
authorized-key file in its `/run` tmpfs and pass its path with the daemon
`-oAuthorizedKeysFile=/run/authorized_keys` override; an arbitrary host-owned bind mount
can instead fail StrictModes ownership checks and does not isolate revocation. Confirm
the server pin and profile digest stayed unchanged and the earlier success is retained.
Remove only this named disposable container afterward:

```bash
docker rm -f opentatu-phase9-ssh-fixture
```

The fixture is not a production deployment recipe. Existing audit/lab tests and prompt
benchmarks retain their own boundaries; phase 9 changes no prompt or model. Phase 10 and
Sentinel monitoring/alerts/response remain separate explicit owner requests.

## Primary references

- [OpenSSH ssh](https://man.openbsd.org/ssh): command invocation and client behavior.
- [OpenSSH ssh_config](https://man.openbsd.org/ssh_config): host-key pins, IdentityAgent,
  configuration expansion, connection limits and disabled local/proxy commands.
- [OpenSSH sshd](https://man.openbsd.org/sshd): restricted authorized-key options.
- [OpenSSH sshd_config](https://man.openbsd.org/sshd_config): ForceCommand, login shell,
  SSH_ORIGINAL_COMMAND, environment and the separate forwarding restriction.


Actual acceptance on 2026-11-13 passed both opt-in tests, the owner CLI positive/revoked-key
flow and direct UID-0 refusal. The synthetic write marker was absent, effective server
configuration retained all restrictions, and the disposable container/listener was removed.
The complete project check passed 312 tests; five opt-in infrastructure tests are ignored
by default, including the two SSH tests that were separately exercised here. See
[ROADMAP.md](ROADMAP.md) for the retained evidence and fixture limitations.
