# NeuralBudget Licensing

NeuralBudget is available under **dual licensing** to serve both open-source and commercial use cases.

## Quick Summary

| License | Cost | Use Case | Commercial Use | Modifications | Distribution |
|---------|------|----------|-----------------|---------------|---------------|
| **Apache 2.0** | Free ✓ | Open source & commercial | ✓ Unlimited | ✓ Yes | ✓ Yes (with attribution) |
| **Commercial** | Paid | Enterprise with support | ✓ Yes | ✓ Yes | ✗ No resale | 

## Apache 2.0 License (Open Source)

**You can use this for free.** Apache 2.0 is an industry-standard permissive license that allows:

### ✓ What You Can Do

- **Use commercially** — Deploy in production, integrate into products, use in business
- **Modify** — Create derivative works and improvements
- **Distribute** — Share your modifications with others
- **Patent protection** — Explicit patent grant from contributors
- **Private use** — Use without public disclosure
- **Sublicense** — Include in your own products

### ✗ Restrictions

- **Give credit** — Retain copyright notices and attribution
- **License propagation** — Distribute with a copy of Apache 2.0 license
- **State changes** — Document significant modifications
- **No trademark** — Don't claim you created NeuralBudget

### Examples of Apache 2.0 Use

**✓ All of these are perfectly legal under Apache 2.0:**

```
1. Use NeuralBudget in your SaaS product
   → Include attribution in docs, ship LICENSE file

2. Modify NeuralBudget for your company's needs
   → Share your improvements back (appreciated but optional)

3. Sell a product that includes NeuralBudget
   → License it as Apache 2.0, keep attribution, profit ✓

4. Integrate into an open-source project
   → Must use compatible license (Apache, MIT, GPL, etc.)

5. Use in a closed-source commercial product
   → Yes! Apache 2.0 allows this (unlike GPL)
```

## Commercial License (Optional)

**Choose Commercial if you need enterprise support.** The Commercial License provides:

### ✓ Commercial License Adds

- **Priority support** — 24-hour response time
- **Custom SLAs** — Guaranteed uptime and reliability guarantees
- **Direct collaboration** — Work directly with the development team
- **Custom features** — Request features prioritized in roadmap
- **Indemnification** — Legal protection for IP infringement claims
- **Dedicated liaison** — Technical account manager for large deployments

### When to Choose Commercial

- You need guaranteed response times
- You want contractual indemnification
- You're deploying at scale (millions of requests)
- You need custom modifications and ongoing support
- You want direct access to the engineering team

### Commercial Pricing

Contact `sales@neuralbudget.io` for pricing:
- **Single Deployment** — Single SLO evaluation system
- **Site License** — Unlimited internal use across organization
- **OEM/Reseller** — Embedding in products (custom terms)

---

## Dependency Compliance

NeuralBudget has **zero GPL dependencies**, allowing you to use NeuralBudget in proprietary products:

### Rust Dependencies (Cargo.toml)

| Dependency | License | Commercial Compatible |
|-----------|---------|----------------------|
| `pyo3` | Apache 2.0 | ✓ Yes |
| `serde` | Apache 2.0 / MIT | ✓ Yes |
| `serde_json` | Apache 2.0 / MIT | ✓ Yes |
| `serde_yaml` | Apache 2.0 / MIT | ✓ Yes |
| `rayon` | Apache 2.0 / MIT | ✓ Yes |
| `criterion` | Apache 2.0 / MIT | ✓ Yes (dev only) |
| `proptest` | Apache 2.0 / MIT | ✓ Yes (dev only) |

### Python Dependencies (pyproject.toml)

NeuralBudget requires Python 3.9+ with optional dependencies:

| Dependency | License | Commercial Compatible |
|-----------|---------|----------------------|
| `pyyaml` | MIT | ✓ Yes (optional) |

**No GPL, AGPL, or Affero licenses.** You're free to use NeuralBudget in proprietary projects.

---

## FAQ: Licensing

**Q: Can I use NeuralBudget in my commercial product?**
> A: Yes! Apache 2.0 explicitly allows commercial use. Just include attribution.

**Q: Do I need to open-source my modifications?**
> A: No. Apache 2.0 is permissive—you can keep modifications private.

**Q: Can I sell a product that uses NeuralBudget?**
> A: Yes, as long as you include a copy of the Apache 2.0 license.

**Q: What if I modify NeuralBudget?**
> A: You can keep modifications private or share them. We appreciate contributions but don't require them.

**Q: Can I remove attribution?**
> A: No. You must retain copyright notices: "Copyright 2026 pristley".

**Q: Is Apache 2.0 GPL-compatible?**
> A: No. Apache 2.0 is incompatible with GPL v2 but compatible with GPL v3.

**Q: When should I buy a Commercial License?**
> A: When you need 24-hour support, SLAs, or want contractual indemnification.

**Q: Can I use NeuralBudget in a closed-source proprietary product?**
> A: Yes! Apache 2.0 allows this (it's permissive). GPL would prohibit it.

**Q: Who owns derivatives I create?**
> A: You do. Apache 2.0 doesn't require you to transfer ownership.

---

## Contributing

We welcome contributions under both licenses:

1. **Open Source Contributors** — Submit PRs licensed under Apache 2.0
2. **Commercial Contributors** — Work directly with the team on priorities

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License History

- **June 2026** — Switched from Source-Available to Apache 2.0 + Commercial dual licensing
- **Prior** — Source-Available License (restrictive organizational use)

---

## Questions?

- **Apache 2.0 Questions** → See http://www.apache.org/licenses/LICENSE-2.0
- **Commercial Licensing** → 
- **License Interpretation** → Open an issue: https://github.com/pristley/NeuralBudget/issues
