{ lib
, buildDotnetModule
, dotnetCorePackages
}:

buildDotnetModule (finalAttrs: {
  pname = "heq";
  version = "1.0.0";

  src = lib.cleanSource ../.;

  projectFile = "src/Heq/Heq.csproj";
  nugetDeps = ./deps.json;

  dotnet-sdk = dotnetCorePackages.sdk_10_0;
  dotnet-runtime = dotnetCorePackages.runtime_10_0;

  runtimeId = "win-x64";
  selfContainedBuild = true;
  executables = [ ]; # heq.exe is a PE binary; there is nothing to wrap on Linux

  dotnetFlags = [ "-p:EnableWindowsTargeting=true" ];

  postInstall = ''
    mkdir -p $out/bin
    ln -s $out/lib/${finalAttrs.pname}/heq.exe $out/bin/heq.exe
  '';

  meta = {
    description = "FabFilter Pro-Q style parametric headphone EQ for Equalizer APO";
    homepage = "https://github.com/tempora/heq";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
    mainProgram = "heq.exe";
  };
})
