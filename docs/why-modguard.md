# Why modguard?

We built `modguard` to solve a particularly painful problem that we ran into time and time again - when developing on a codebase with others, the intended interface for a given module would eventually get violated and become a dumping ground for all loosely related code. Imports would tightly couple together what used to be independent modules, and create "mega-modules" of code sprawl. These would in turn be harder to test and significantly more painful to make changes to. Additionally, mis-use of existing modules would degrade performance and cause security incidents.

This would happen for a variety of reasons:
- Junior or new developers had a poor understanding of the existing architecture and/or frameworks being used
- It's significantly easier to add to an existing service rather than create a new one
- Python doesn't prohibit you from importing or using any code living anywhere
- When changes are in a 'gray area', social desire to not block others would let changes through code review
- External deadlines and management pressure would result in "doing it properly" getting punted and/or never done

The attempts to fix this problem almost always came up short. Inevitably, standards guides would be written and stricter and stricter attempts would be made to enforce style guides, lead developer education efforts, and restrict code review. However, each of these approaches had their own flaws. 

The solution was to create a set of definitions for each module and it's public interface, and enforce those domain boundaries through CI. This meant that no developer could ever violate these boundaries without explicitly changing the definition of either the interface or the boundary, a significantly smaller and well scoped set of changes that could then be maintained and managed by those who understood the correct semantics for the system.

With modguard set up, you can collaborate on your codebase with confidence that developers won't violate the intentional design of your modules.