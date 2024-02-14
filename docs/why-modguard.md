# Why modguard?

We built `modguard` to solve a recurring problem that we've experienced on software teams - code sprawl. Unintended cross-module imports would tightly couple together what used to be independent domains, and eventually create "balls of mud". This makes it harder to test, and harder to make changes. Not to mention that mis-use of modules which were intended to be private can degrade performance and cause security incidents.

This would happen for a variety of reasons:
- Junior or new developers had a poor understanding of the existing architecture and/or frameworks being used
- It's significantly easier to add to an existing service than to create a new one
- Python doesn't stop you from importing any code living anywhere
- When changes are in a 'gray area', social desire to not block others would let changes through code review
- External deadlines and management pressure would result in "doing it properly" getting punted and/or never done

The attempts to fix this problem almost always came up short. Inevitably, standards guides would be written and stricter and stricter attempts would be made to enforce style guides, lead developer education efforts, and restrict code review. However, each of these approaches had their own flaws. 

The solution was to create a set of definitions for each module and it's public interface, and enforce those domain boundaries through CI. This meant that no developer could ever violate these boundaries without explicitly changing the definition of either the interface or the boundary, a significantly smaller and well scoped set of changes that could then be maintained and managed by those who understood the correct semantics for the system.

With modguard set up, you can collaborate on your codebase with confidence that developers won't violate the intentional design of your modules.
Modguard is:
- fully open source
- able to be adopted incrementally
- implemented with no runtime footprint
- a standalone library with no external dependencies
- interoperable with your existing system (cli, generated config)