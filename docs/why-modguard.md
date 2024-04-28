# Why modguard?

## The Problem
We built `modguard` to solve a recurring problem that we've experienced on software teams -- **code sprawl**. Unintended cross-package imports would tightly couple together what used to be independent domains, and eventually create **"balls of mud"**. This made it harder to test, and harder to make changes. Mis-use of packages which were intended to be private would then degrade performance and even cause security incidents.

This would happen for a variety of reasons:

- Junior developers had a limited understanding of the existing architecture and/or frameworks being used
- It's significantly easier to add to an existing service than to create a new one
- Python doesn't stop you from importing any code living anywhere
- When changes are in a 'gray area', social desire to not block others would let changes through code review
- External deadlines and management pressure would result in "doing it properly" getting punted and/or never done

The attempts to fix this problem almost always came up short. Inevitably, standards guides would be written and stricter and stricter attempts would be made to enforce style guides, lead developer education efforts, and restrict code review. However, each of these approaches had their own flaws. 

## The Solution
The solution was to explicitly define a package's **boundary** and **public interface** in code, and enforce those domain boundaries through CI. This meant that no developer could introduce a new cross-package dependency without explicitly changing the public interface or the boundary itself. This was a significantly smaller and well-scoped set of changes that could be maintained and managed by those who understood the intended design of the system.

With `modguard` set up, you can collaborate on your codebase with confidence that the intentional design of your packages will always be preserved.

`modguard` is:

- fully open source
- able to be adopted incrementally
- implemented with no runtime footprint
- interoperable with your existing CI tools