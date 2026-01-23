# rstask

rstask is a Rust port of [dstask](https://github.com/naggie/dstask), which is described as "a personal task tracker designed to help you focus".

## Compatibility

rstask implements all of dstask apart from the ability to import from github / taskwarrior.

In general, renaming your `.dstask` folder to `.rstask` will work. Though, you will lose the current context.

## Future

I eventually do not intend this project to merely be a Rust copy of dstask. I think dstask has some absolute incredible ideas, taking honestly to me the best out of taskwarrior and adding the Git sync. However in other areas, I think there's several improvements possible: The UI is a bit wonky, it doesn't have the ecosystem support that taskwarrior has so it's lacking in integrations, and I think in today's era of AI and what not, it's possible to turn it into a more convivial experience.

I do not like Go at all, and maintenance of dstask has been a bit spotty at time (which is understandable after so many years!), so I figured I'd just fork and make it my own. It's possible that with time compatibility with dstask is lost or degraded as new features are added.

## License

[MIT](./LICENSE). Most of the code is severely inspired from dstask, which is [also under MIT](https://github.com/naggie/dstask/blob/master/LICENSE).
