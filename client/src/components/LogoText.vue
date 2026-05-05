<template>
    <span class="dt-brand" aria-label="DigTrax">
        <span class="dt-dig">DIG</span><span class="dt-trax">TRAX</span>
    </span>
</template>

<style scoped>
.dt-brand {
    font-family: 'JetBrains Mono', 'SF Mono', monospace;
    font-weight: 700;
    font-size: 22px;
    letter-spacing: 0.02em;
    line-height: 42px;
    display: inline-flex;
    align-items: center;
    height: 42px;
    user-select: none;
}

.dt-dig {
    color: var(--color-fg, #ffffff);
}

.dt-trax {
    color: var(--color-accent, #00D2BF);
    text-shadow: 0 0 16px var(--color-accent-glow, rgba(0, 210, 191, 0.6));
}
</style>
