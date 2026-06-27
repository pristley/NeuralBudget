# NeuralBudget Licensing

NeuralBudget is licensed under **Apache 2.0** — a permissive, industry-standard open-source license.

## Quick Summary

| License | Cost | Use Case | Commercial Use | Modifications | Distribution |
|---------|------|----------|-----------------|---------------|---------------|
| **Apache 2.0** | Free ✓ | Open source, SaaS, enterprise | ✓ Unlimited | ✓ Yes | ✓ Yes (with attribution) | 

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

## Why Apache 2.0?

Apache 2.0 provides the perfect balance:
- **Permissive** — Use commercially without restrictions
- **Business-friendly** — Clear terms for enterprise adoption
- **Patent protection** — Explicit patent grants included
- **Derivative-friendly** — Modify freely for your needs
- **Attribution-light** — Only requires license notice in distribution

No GPL restrictions. No commercial licensing fees. Just open source the way it should be.

---

## Dependency Compliance

NeuralBudget has **zero GPL dependencies**, allowing you to use NeuralBudget in proprietary products:

### Rust Dependencies (Cargo.toml)

| Dependency | License |
|-----------|---------|
| `pyo3` | Apache 2.0 |
| `serde` | Apache 2.0 / MIT |
| `serde_json` | Apache 2.0 / MIT |
| `serde_yaml` | Apache 2.0 / MIT |
| `rayon` | Apache 2.0 / MIT |
| `criterion` | Apache 2.0 / MIT (dev only) |
| `proptest` | Apache 2.0 / MIT (dev only) |

### Python Dependencies (pyproject.toml)

NeuralBudget requires Python 3.9+ with optional dependencies:

| Dependency | License |
|-----------|---------|
| `pyyaml` | MIT (optional) |

**No GPL, AGPL, or Affero licenses.** You're free to use NeuralBudget in proprietary projects.

---

## FAQ: Apache 2.0 Licensing

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

**Q: Can I use NeuralBudget in a closed-source proprietary product?**
> A: Yes! Apache 2.0 allows this (it's permissive). GPL would prohibit it.

**Q: Who owns derivatives I create?**
> A: You do. Apache 2.0 doesn't require you to transfer ownership.

---

## Contributing

We welcome contributions under Apache 2.0!

1. **Open Source Contributors** — Submit PRs licensed under Apache 2.0
2. All contributions must be compatible with Apache 2.0

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License History

- **June 2026** — Switched to Apache 2.0 open-source license
- **Prior** — Source-Available License (restrictive organizational use)

---

## Questions?

- **Apache 2.0 Questions** → See http://www.apache.org/licenses/LICENSE-2.0
- **Commercial Licensing** → 
- **License Interpretation** → Open an issue: https://github.com/pristley/NeuralBudget/issues
