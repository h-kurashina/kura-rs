import os

from hypothesis import settings

# 既定は 1,000 通り。CI では KURA_HYPOTHESIS_EXAMPLES で増やす
settings.register_profile("kura", max_examples=int(os.environ.get("KURA_HYPOTHESIS_EXAMPLES", "1000")), deadline=None)
settings.load_profile("kura")
