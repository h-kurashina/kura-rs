"""Aho–Corasick search for many patterns in one pass, checked against pyahocorasick.

    from kura_rs import multi_pattern_match

    matcher = multi_pattern_match.Matcher(["evil.example", "198.51.100.7"])
    for pattern_id, start, end in matcher.find_all(log_line):
        print(matcher.patterns[pattern_id], start, end)
"""

from kura_rs._native import multi_pattern_match as _native

Matcher = _native.Matcher

__all__ = ["Matcher"]
