// Joins horizon fields (HZN2) into one file: the places of each, in order.
//
//   node scripts/merge-fields.ts OUT.hzn IN.hzn [IN.hzn …]
import { readFileSync, writeFileSync } from "node:fs";

const [outPath, ...inPaths] = process.argv.slice(2);
if (!outPath || inPaths.length === 0) {
	console.error(
		"usage: node scripts/merge-fields.ts OUT.hzn IN.hzn [IN.hzn …]",
	);
	process.exit(1);
}
let azimuths = 0;
let count = 0;
const bodies = inPaths.map((path) => {
	const data = readFileSync(path);
	if (data.toString("latin1", 0, 4) !== "HZN2")
		throw new Error(`${path} is not an HZN2 field`);
	const az = data.readInt32LE(8);
	if (azimuths && az !== azimuths)
		throw new Error(`${path} has ${az} azimuths, not ${azimuths}`);
	azimuths = az;
	count += data.readInt32LE(4);
	return data.subarray(12);
});
const header = Buffer.alloc(12);
header.write("HZN2", 0, "latin1");
header.writeInt32LE(count, 4);
header.writeInt32LE(azimuths, 8);
writeFileSync(outPath, Buffer.concat([header, ...bodies]));
console.log(`wrote ${count} places from ${inPaths.length} fields → ${outPath}`);
