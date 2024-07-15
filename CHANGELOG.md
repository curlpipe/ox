# Kaolinite Changelog
All dates are in DD/MM/YYYY format. 

This project uses semantic versioning.

<!--
## [X.Y.Z] - DD/MM/YYYY
\+
\~
\-
-->
## [0.8.0] - 15/07/2024
\+ Added "new document" feature to allow documents without file names

## [0.7.0] - 25/02/2024
\+ Nice integration with synoptic
\~ Fixed bug that stopped empty files from being edited

## [0.6.1] - 30/08/2022
\~ Updated cargo.toml

## [0.6.0] - 30/08/2022
\~ Revamped the entirety of kaolinite

\+ Added file buffering

\+ Added Searching and replacing

\- Separated syntax highlighting from kaolinite (to be used externally)

## [0.5.0] - 09/07/2021
\+ Added undo and redo

\+ Added insert and remove line commands to cactus

\~ Changed several events to properly allow reversing of them

\~ Fixed a few regex expressions for cactus syntax highlighting

\- Removed feature system, all are enabled by default

## [0.4.1] - 08/07/2021
\~ Updated documentation

\- Removed heavy logo files

## [0.4.0] - 08/07/2021
\+ Added syntax highlighting helper feature

\+ Added syntax highlighting using `synoptic` to cactus.

\+ Added `render_full` method to render the entire document in display form

\~ Changed API to remove unsafe code and pointer hell

## [0.3.2] - 06/07/2021
\+ Added alignment helper

\+ Added status line formatting helper

\+ Added line wrapping to cactus

\~ Simplified cactus code

## [0.3.1] - 06/07/2021
\~ Fixed panic issues in next_word_forth

## [0.3.0] - 06/07/2021
\+ Added cactus: a editor to demonstrate kaolinite

\+ Added support for accessing the line below the document

\+ Added a method to generate line number text

\+ Added support for tab rendering

\+ Added methods for finding the next and previous word index

\+ Added functions to help with display widths

\+ Added file type lookup function to determine type from file extension

\~ Fixed issues with removing

\~ Fixed issues with splicing up

\~ Used char indices instead of display indices

\~ Fixed the EOI issues

\~ Followed clippy lints

## [0.2.1] - 30/06/2021
\~ Row linking optimisation (~1.33x faster)

## [0.2.0] - 30/06/2021
\~ Text removal optimisation (~1.25x faster)

\- Only allowed inclusive and exclusive ranges in Row::remove to prevent spaghettification

## [0.1.5] - 30/06/2021
\~ Row splicing optimisation (~1.2x faster)

## [0.1.4] - 30/06/2021
\~ More huge optimisation (~5x faster)

## [0.1.3] - 30/06/2021
\~ Huge optimisation (~3x faster)

## [0.1.2] - 29/06/2021
\+ Added benchmark

## [0.1.1] - 29/06/2021
\+ Added changelog

## [0.1.0] - 29/06/2021
\+ Initial release
