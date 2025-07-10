# lst - Todos and Lists for humans and agents

## paradigm

Everything is text, everything is a file. The source of truth are the markdown files (defined in the lst.toml in ~/.config/lst/)
We sync everything encrypted to a server as crdt so that we can work on multiple devices. We have a simple sharing system that makes it possible to live sync files with other users.

The mobile app has its own sqlite database due to constraints but all other clients use the plain markdown files as the source of truth.

