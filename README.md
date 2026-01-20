# Summary
Floatglass is an attempt to make a cross platform windows media creation tool, with current aims to support MacOS and linux.

It is not currently in an operational state and is under active
development.

# Supported platforms
### Tier 1
> These platforms are the top priority, and the motivation behind
> the project's existence.
- MacOS (ARM & x86_64)
- Linux (x86_64)

### Tier 2
> There are no plans to support these platforms at this time, but support
> is possible and has plausible value.
- Android (possible, not currently planned)
- iPadOS (*maybe* possible, not currently planned)
- Windows (due to the nature of the tech stack chosen, support for windows might be as easy as compiling for windows)

### Unsupported
> It is difficult or impossible to support these platforms.
- Web (WebUSB does not provide the APIs necessary and will likely never
  do so)
- iOS (impossible, there's no way to interface directly with a USB device
  through iOS without rooting the device)

# How does it work?
Floatglass is built around a cross platform USB library, and interacts
with USB drives as block devices.

Rufus is not cross platform and has no plans to be so, and you can't use
other burning utilities because windows is quirky and the provided iso is
an NTFS disk image, and so you need an NTFS driver burned onto the
partition table, which then bootstraps into the windows installer.
The Rufus project maintains an NTFS driver I intend to utilize.

# Tech stack
The core of the project will be in Rust, GUI is TBD.

Contributions welcome, please ask if you have questions

# LLM Contribution Policy
This project's LLM contributor policy is borrowed from LLVM
(<https://llvm.org/docs//AIToolPolicy.html>):
> [..] contributors can use whatever tools they would
> like to craft their contributions, but there must be a human in the
> loop. Contributors must read and review all LLM-generated code or text
> before they ask other project members to review it. The contributor is
> always the author and is fully accountable for their contributions. 
> Contributors should be sufficiently confident that the contribution is
> high enough quality that asking for a review is a good use of scarce
> maintainer time, and they should be able to answer questions about
> their work during review.

> Contributors are expected to be transparent and label contributions
> that contain substantial amounts of tool-generated content. Our policy
> on labelling is intended to facilitate reviews, and not to track which
> parts of [code] are generated. Contributors should note tool usage in
> their pull request description, commit message, or wherever authorship
> is normally indicated for the work. For instance, use a commit message
> trailer like Assisted-by: . This transparency helps the community
> develop best practices and understand the role of these new tools.
