// openapi-typescript 7 drives the TypeScript compiler API, which TypeScript 7 (the native compiler) does not
// ship. The generator gets its own TypeScript 5.9.3; the app is still checked with 7.0.2.
module.exports = {
  hooks: {
    readPackage(pkg) {
      if (pkg.name === "openapi-typescript") {
        if (pkg.peerDependencies) delete pkg.peerDependencies.typescript;
        pkg.dependencies = { ...pkg.dependencies, typescript: "5.9.3" };
      }
      return pkg;
    },
  },
};
