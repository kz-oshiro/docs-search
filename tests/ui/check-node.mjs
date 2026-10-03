if (Number(process.versions.node.split('.')[0]) < 20) {
  console.error(`Node.js 20 or newer is required (found ${process.version}).`);
  process.exitCode = 1;
}
