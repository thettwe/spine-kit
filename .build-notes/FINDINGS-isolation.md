# Two defects in `result-file.md` §7.1, found by running M1 rather than reading it

Host: Docker Desktop 28.5.1 on darwin/arm64, Linux **6.11.11-linuxkit** aarch64,
container `postgres:18` (Debian bookworm) run `--privileged`, collector-equivalent
running as uid 0 — RF §7.1's identity arrangement 2, "a root collector that drops
privilege", which is the arrangement the spec calls "the ordinary case on a
container-based CI runner".

Both defects have the shape the corpus has already diagnosed twice and fixed
twice — a probe that **fails on every host, for every configuration, forever** —
and both are terminal in the same way: P3 or P4 failing means `profile=container`
may never be written (RF §7.1 step 6), so **every landing in every repository**
records `profile=none`, fails auto-merge precondition 1 on that fact alone, and
raises the `class=tripwire` `G11` wire (PB §7.4 rule 5). Auto-merge becomes
unreachable, not degraded.

The corpus caught this shape for P2 — *"an unprivileged user namespace with no
delegated subordinate range … maps exactly one host uid, `U` itself, so the file
P2 `stat`s comes back owned by `U` and the test fails on every host, for every
configuration, forever"* — and for P3 — *"A mount namespace over the job's root
*is* the job's root … so P3's separation limb would fail on every host, for every
configuration, forever."* Neither of the two below was caught, and neither is
visible from the text: they are facts about the kernel, which is exactly the
category RF §7.1 exists to test rather than assume.

---

## Defect 1 — M1's root shape does not mount. An upperdir-less overlay needs two lower layers.

**What the spec says.** RF §7.1, *M1's root — an overlay, and why not the job's
root itself*:

> Inside the mount namespace the collector mounts an **overlay** whose only layer
> is a **lower** layer: the job's own root filesystem.

**What the kernel does.** An overlay with no `upperdir` and exactly one
`lowerdir` is refused with `EINVAL`. Two lower layers are required. Measured:

| `mount -t overlay overlay -o …` | result |
|---|---|
| `lowerdir=/run/t/low` | **FAIL** — `wrong fs type, bad option, bad superblock` |
| `lowerdir=/run/t/low:/run/t/empty` | PASS |
| `lowerdir=/` | **FAIL** |
| `lowerdir=/:/run/t/empty` where `empty` is an ordinary directory under `/` | **FAIL** — `ELOOP, Too many levels of symbolic links` |

The second row is the kernel's actual rule; the fourth is the trap waiting for
anyone who reads the second and reaches for the obvious fix, because a second
lower layer that is *reachable through the first* is a recursion the kernel
refuses.

This is not a Docker artefact. Every variant failed identically as **root, with
no user namespace at all**, and with a `tmpfs` lower layer, so it is neither
"unprivileged overlayfs unavailable" nor "a lower layer the kernel refuses to
stack" — the two absences RF §7.1's prerequisite 3 already names. It is the
single-lower-layer rule, which no prerequisite names.

**A shape that works.** The second lower layer must be an empty directory on its
own filesystem, mounted before the overlay:

```sh
mount -t tmpfs tmpfs /run/spine/empty
mount -t overlay overlay -o lowerdir=/:/run/spine/empty /run/spine/root
```

Measured on the same host — and note the third line, which is the whole of what
P3's separation limb asks for:

```
lowerdir=/:<tmpfs empty>            PASS
overlay   dev=97  ino=2
hostroot  dev=79  ino=34252584      <- different (device, inode) pair: P3 passes
read-only as M1 requires            <- touch inside it fails
contents: bin boot dev etc home lib media mnt …
```

`lowerdir=<bind of />:<tmpfs empty>` and `lowerdir=<tmpfs empty>:/` also mount,
also read-only, also with a distinct `(device, inode)`. The ordering of the two
lower layers does not matter here because the second is empty — which is the
reason to require it empty rather than merely to require a second.

**Recommended amendment.** RF §7.1's overlay paragraph gains one sentence: the
overlay's lower set is the job's root **and one empty directory on a separate
filesystem the collector mounts for the purpose**, because the kernel refuses an
upperdir-less overlay with a single lower layer, and refuses a second lower layer
reachable through the first. Prerequisite 3 gains the corresponding absence: *no
filesystem the collector may mount a `tmpfs` on*.

---

## Defect 2 — P4(a) is unpassable on any kernel with the tunnel modules loaded.

**What the spec says.** RF §7.1, the P4 row (`result-file.md:386`), verbatim:

> **(a)** the interface set is **exactly one device, loopback**, carrying no
> address other than `127.0.0.1/8` and `::1/128`

and, in *M1 — the shipped mechanism* (`result-file.md:328`):

> It is fresh, it is **empty but for a `lo` device** the collector brings up

**What the kernel does.** A fresh network namespace is not empty but for `lo`.
Where the `ipip`, `gre`, `sit` and `ip6_tunnel` modules are loaded — the default
on essentially every distribution kernel, including CI runner images — the kernel
instantiates a per-namespace device for each one at namespace creation. Measured
inside `unshare --net --user --map-root-user`:

```
device count from /proc/net/dev: 10
devices: lo tunl0 gre0 gretap0 erspan0 ip_vti0 ip6_vti0 sit0 ip6tnl0 ip6gre0

  lo       flags=0x9     (IFF_UP | IFF_LOOPBACK)
  tunl0    flags=0x80    (IFF_NOARP)          — down
  gre0     flags=0x80                          — down
  gretap0  flags=0x1002  (IFF_BROADCAST|IFF_MULTICAST) — down
  erspan0  flags=0x1002                        — down
  ip_vti0  flags=0x80                          — down
  ip6_vti0 flags=0x80                          — down
  sit0     flags=0x80                          — down
  ip6tnl0  flags=0x80                          — down
  ip6gre0  flags=0x80                          — down

IPv4 addresses in this netns: (none)
IPv6 addresses in this netns: (none)
```

Nine devices beyond loopback, every one **down**, every one carrying **no
address**. The namespace is as isolated as the spec intends — the boundary is
sound — but the literal test P4(a) states fails on it, and therefore forever.

**The sound half is already in the spec.** *"carrying no address other than
`127.0.0.1/8` and `::1/128`"* is exactly right and passes here: the address set
of that namespace is precisely loopback's. It is the *device-count* clause that
is wrong, and it is wrong because it describes an intent ("nothing but loopback")
in terms of an artefact the kernel does not honour.

**Recommended amendment.** P4(a)'s pass condition becomes: **no interface other
than loopback is `IFF_UP`, and no address of any family exists in the namespace
other than `127.0.0.1/8` and `::1/128`.** That is strictly stronger than the
device count against the threat P4 exists to detect — a veth pair moved in, a
bridge, an inherited interface — because any of those must be up and addressed to
carry traffic, while a down, address-less `gre0` cannot.

---

## A third thing, not a defect: where P4(a) must read from

RF §7.1 warns that *"a collector could read it from the wrong namespace"* and
this is not hypothetical. Measured inside the same fresh network namespace:

```
/proc/net/dev  : lo tunl0 gre0 gretap0 erspan0 ip_vti0 ip6_vti0 sit0 ip6tnl0 ip6gre0
/sys/class/net : bonding_masters erspan0 eth0 gre0 gretap0 ip6gre0 ip6tnl0 ip…
```

`/sys/class/net` still lists **`eth0`** — the job's own external interface —
because `sysfs` was mounted in the old network namespace and is not re-derived by
`unshare(2)`. A probe that enumerates interfaces through `sysfs` without
remounting it reports the *host's* interface set from inside a correctly isolated
namespace, which is a false **fail**; the mirror-image bug, a probe that trusts a
`sysfs` mounted by a runtime that did re-derive it, would be a false **pass**.

The implementation reads the interface and address set over **netlink
(`RTM_GETLINK` / `RTM_GETADDR`)** from inside the probe, which is answered by the
namespace the calling socket belongs to and cannot be aimed at another one.
`/proc/net/dev` and `/proc/net/if_inet6` are correct too — procfs net files are
resolved per-namespace from the reading task — and are what the measurements
above used.

---

## Status

Neither defect is a blocker for building: both have a shape that works and both
fixes are local. They are recorded here because they are **owner amendments to a
normative spec**, not implementation choices — `result-file.md` §7.1 is what two
independent collectors must agree on, and a collector built to the text as it
stands today would be a conforming collector that can never write
`profile=container`.

Reproduce everything above with the scripts in the session scratchpad
(`m1.sh` … `m4.sh`), or from the commands quoted inline.
