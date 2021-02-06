var nodeExternals = require("webpack-node-externals");
const path = require("path");

module.exports = {
  target: "node", // in order to ignore built-in modules like path, fs, etc.
  externals: [
    nodeExternals({
      // Get modules from package.json file not from node_modules folder: https://github.com/liady/webpack-node-externals/issues/39#issuecomment-431137763
      modulesFromFile: true,
    }),
  ],
  entry: {
    app: "./src/app/index.tsx",
    main: "./src/main/main.ts",
  },
  devtool: "inline-source-map",
  output: {
    path: __dirname + "/build",
    filename: "[name]-bundle.js",
    // Bundle absolute resource paths in the source-map,
    // so VSCode can match the source file.
    devtoolModuleFilenameTemplate: "[absolute-resource-path]",
  },
  resolve: {
    extensions: [".ts", ".tsx", ".js"],
    alias: {
      Assets: path.resolve(__dirname, "assets"),
    },
  },
  node: {
    __dirname: false,
    __filename: false,
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
      {
        test: /\.s?css$/,
        use: [
          "style-loader",
          { loader: "css-loader", options: { url: false } },
          "sass-loader",
        ],
      },
      {
        test: /\.(eot|svg|ttf|woff|woff2)$/,
        loader: "file-loader",
        options: {
          name: "fonts/[name].[ext]",
        },
      },
      {
        test: /\.node$/,
        loader: "native-ext-loader",
      },
    ],
  },
};
