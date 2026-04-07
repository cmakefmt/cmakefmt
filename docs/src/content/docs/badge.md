---
title: Badge
description: Add a "Formatted with cmakefmt" badge to your project README.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Show that your project uses `cmakefmt` by adding a badge to your `README`.

## Badge

[![Formatted with cmakefmt](https://img.shields.io/badge/formatted%20with-cmakefmt-blue)](https://cmakefmt.dev)

## Markdown

```markdown
[![Formatted with cmakefmt](https://img.shields.io/badge/formatted%20with-cmakefmt-blue)](https://cmakefmt.dev)
```

## reStructuredText

```rst
.. image:: https://img.shields.io/badge/formatted%20with-cmakefmt-blue
   :target: https://cmakefmt.dev
   :alt: Formatted with cmakefmt
```

## HTML

```html
<a href="https://cmakefmt.dev">
  <img src="https://img.shields.io/badge/formatted%20with-cmakefmt-blue"
       alt="Formatted with cmakefmt" />
</a>
```

## Colour variants

The badge uses the default shields.io blue. You can swap the colour by
replacing `blue` in the URL with any [named shields.io colour](https://shields.io/docs/colors)
or a hex code (without `#`), for example:

```
https://img.shields.io/badge/formatted%20with-cmakefmt-4a90d9
```
