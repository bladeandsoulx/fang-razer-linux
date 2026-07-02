<script>
  import { icons } from '../icons.js';
  export let name;
  export let size = 20;
  export let stroke = 1.8;
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <path d={icons[name]} />
</svg>
