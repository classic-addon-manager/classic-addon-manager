export namespace addon {
	
	export class Addon {
	    name: string;
	    alias: string;
	    description: string;
	    version: string;
	    commit: string;
	    author: string;
	    repo: string;
	    isManaged: boolean;
	    // Go type: time
	    updatedAt: any;
	
	    static createFrom(source: any = {}) {
	        return new Addon(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.alias = source["alias"];
	        this.description = source["description"];
	        this.version = source["version"];
	        this.commit = source["commit"];
	        this.author = source["author"];
	        this.repo = source["repo"];
	        this.isManaged = source["isManaged"];
	        this.updatedAt = this.convertValues(source["updatedAt"], null);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class AddonManifest {
	    name: string;
	    alias: string;
	    description: string;
	    author: string;
	    repo: string;
	    branch: string;
	    tags: string[];
	    downloads: number;
	    like_percentage?: number;
	    kofi?: string;
	
	    static createFrom(source: any = {}) {
	        return new AddonManifest(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.alias = source["alias"];
	        this.description = source["description"];
	        this.author = source["author"];
	        this.repo = source["repo"];
	        this.branch = source["branch"];
	        this.tags = source["tags"];
	        this.downloads = source["downloads"];
	        this.like_percentage = source["like_percentage"];
	        this.kofi = source["kofi"];
	    }
	}

}

export namespace api {
	
	export class ApplicationRelease {
	    version: string;
	    url: string;
	
	    static createFrom(source: any = {}) {
	        return new ApplicationRelease(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.version = source["version"];
	        this.url = source["url"];
	    }
	}
	export class Tag {
	    ref: string;
	    sha: string;
	    type: string;
	    url: string;
	
	    static createFrom(source: any = {}) {
	        return new Tag(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.ref = source["ref"];
	        this.sha = source["sha"];
	        this.type = source["type"];
	        this.url = source["url"];
	    }
	}
	export class Release {
	    zipball_url: string;
	    tag_name: string;
	    body: string;
	    // Go type: time
	    published_at: any;
	    tag: Tag;
	
	    static createFrom(source: any = {}) {
	        return new Release(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.zipball_url = source["zipball_url"];
	        this.tag_name = source["tag_name"];
	        this.body = source["body"];
	        this.published_at = this.convertValues(source["published_at"], null);
	        this.tag = this.convertValues(source["tag"], Tag);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

export namespace github {
	
	export class GithubTag {
	    ref: string;
	    sha: string;
	    type: string;
	    url: string;
	
	    static createFrom(source: any = {}) {
	        return new GithubTag(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.ref = source["ref"];
	        this.sha = source["sha"];
	        this.type = source["type"];
	        this.url = source["url"];
	    }
	}
	export class GithubRelease {
	    zipball_url: string;
	    tag_name: string;
	    body: string;
	    // Go type: time
	    published_at: any;
	    tag: GithubTag;
	
	    static createFrom(source: any = {}) {
	        return new GithubRelease(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.zipball_url = source["zipball_url"];
	        this.tag_name = source["tag_name"];
	        this.body = source["body"];
	        this.published_at = this.convertValues(source["published_at"], null);
	        this.tag = this.convertValues(source["tag"], GithubTag);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

export namespace util {
	
	export class LogParseResult {
	    Type: string;
	    Addon: string;
	    File: string;
	    Error: string;
	
	    static createFrom(source: any = {}) {
	        return new LogParseResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.Type = source["Type"];
	        this.Addon = source["Addon"];
	        this.File = source["File"];
	        this.Error = source["Error"];
	    }
	}

}

